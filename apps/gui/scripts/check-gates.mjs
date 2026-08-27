// The gate over the gates: every check in `scripts/` can be started by
// name, and every gate that can be started is written down.
//
// **Why this exists at all.** 2026-08-27 three gates — `check-tabs`,
// `smoke`, `web-face` — had no `npm run` entry. They worked; nothing
// could start them by name. `docs/DESIGN.md` 四 pointed at one of them
// as「守门在 scripts/check-tabs.mjs 第 0 节」while `npm run tabs`
// answered `Missing script: "tabs"`. The same afternoon `docs/DESIGN.md`
// 二 was titled「两道守门」while five entries existed, and somebody
// (me) wrote「守门都绿」in a commit message having run two of them.
//
// Both halves of that are the same failure with the same shape: **a gate
// nobody can reach, and a gate nobody knows about, are worth the same as
// no gate.** Neither half is visible to any of the other checks — they
// all read `src/`, and this one reads `package.json` and the doc.
//
// Claims:
//   1. no orphan: every `.mjs` under `scripts/` is started by an npm
//      script, or imported by one that is (a shared helper is reachable
//      through its importer; a file nothing reaches is either a gate
//      that cannot be run or dead code, and both deserve the red);
//   2. no dead entry: every `scripts/…` path an npm script names is a
//      file that exists — a typo'd entry fails at the moment somebody
//      trusts it, which is the worst moment;
//   3. no silent gate: every npm script that starts one of these is
//      named in `docs/DESIGN.md`, so the list in the doc cannot fall
//      behind the list in `package.json` without going red. Matching is
//      on the entry name (`npm run face`), not on prose, so rewriting
//      the section around it is free and adding a gate is not.
//
// **What it cannot block:** a gate that is reachable, documented, and
// asserts nothing — only reading it catches that. And claim 3 checks
// that a gate is *mentioned*, never that the mention is true. That is
// not a hypothetical: on this file's first run `npm run tabs` passed
// claim 3 while the only sentence containing it in the doc said
// `npm error Missing script: "tabs"` — **a mention that asserted the
// gate did not exist satisfied「the gate is written down」**. Substring
// matching buys「somebody typed this name here」and nothing beyond it.
//
// This file is its own first customer: it needs an `npm run gates` entry
// to satisfy claim 1, and its own name in the doc to satisfy claim 3.
//
// Run: `npm run gates` (static, no browser, no cargo lock).
import { readdirSync, readFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const gui = fileURLToPath(new URL("..", import.meta.url));
const scriptsDir = join(gui, "scripts");
const doc = join(gui, "..", "..", "docs", "DESIGN.md");

let bad = 0;
const fail = (m) => {
  console.error(m);
  bad += 1;
};

const pkg = JSON.parse(readFileSync(join(gui, "package.json"), "utf8"));
const npmScripts = pkg.scripts ?? {};
const onDisk = readdirSync(scriptsDir).filter((f) => f.endsWith(".mjs"));

// The probe, before any negative claim: if the parse found nothing, every
// assertion below passes by vacuum. That reads as「all clean」and means
// 「I looked in the wrong place」— the shape this repo keeps meeting.
if (Object.keys(npmScripts).length === 0) {
  fail("probe dead: package.json declares no scripts — this gate read the wrong file");
} else if (onDisk.length === 0) {
  fail(`probe dead: no .mjs found under ${scriptsDir} — this gate read the wrong directory`);
} else if (!existsSync(doc)) {
  fail(`probe dead: ${doc} is not there — claim 3 would pass on every gate`);
}

// Which npm entries start a script here, and which file each one names.
const NAMES = /scripts[/\\]([\w.-]+\.mjs)/g;
const starts = new Map(); // entry name → [file, …]
for (const [name, body] of Object.entries(npmScripts)) {
  const files = [...String(body).matchAll(NAMES)].map((m) => m[1]);
  if (files.length) starts.set(name, files);
}

// ── 2) no dead entry ───────────────────────────────────────────────
for (const [name, files] of starts) {
  for (const f of files) {
    if (!existsSync(join(scriptsDir, f))) {
      fail(`dead entry: \`npm run ${name}\` runs scripts/${f}, which is not on disk`);
    }
  }
}

// ── 1) no orphan, following imports one hop at a time ──────────────
const reached = new Set([...starts.values()].flat());
for (let grew = true; grew; ) {
  grew = false;
  for (const f of [...reached]) {
    const p = join(scriptsDir, f);
    if (!existsSync(p)) continue;
    const src = readFileSync(p, "utf8");
    for (const m of src.matchAll(/from\s+"\.\/([\w.-]+\.mjs)"/g)) {
      if (!reached.has(m[1])) {
        reached.add(m[1]);
        grew = true;
      }
    }
  }
}
for (const f of onDisk) {
  if (reached.has(f)) continue;
  fail(
    `orphan gate: scripts/${f} is on disk and nothing starts it — add an ` +
      `\`npm run\` entry for it, or have a script that has one import it. ` +
      `A gate that cannot be started by name is worth what no gate is worth.`,
  );
}

// ── 3) no silent gate ──────────────────────────────────────────────
const said = existsSync(doc) ? readFileSync(doc, "utf8") : "";
for (const name of starts.keys()) {
  // `pretest`/`prebuild` and friends are npm's own lifecycle hooks: they
  // are not something a person chooses to run, so the doc owes them
  // nothing. Named positively — a `pre`-prefixed entry whose base exists.
  if (name.startsWith("pre") && npmScripts[name.slice(3)]) continue;
  if (said.includes(`npm run ${name}`)) continue;
  fail(
    `silent gate: \`npm run ${name}\` starts ${starts.get(name).join(", ")} ` +
      `and docs/DESIGN.md never names it. Put it in the table in 二 — a gate ` +
      `nobody knows about is one nobody runs, which is how「守门都绿」gets ` +
      `written about two of five.`,
  );
}

if (bad) process.exit(1);
console.log(
  `gates: ${onDisk.length} scripts, ${starts.size} entries — each one startable by name and named in the doc`,
);
