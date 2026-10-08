// The words gate (WORDS-1008, the owner 2026-10-08): one way to say a
// machine's connection, a time, a latency and an amount of money, and
// every page saying it through `src/words.ts`.
//
// The owner: 「连不上叫不通; MDH这个只在需要缩写的地方吧, 比如额度窗口那,
// 5D窗口. 普通的还是2秒前这样.重置如2小时13分后重置.」 The audit (items 7
// and 8) found five words for 「cannot be reached」, a filter that did not
// find the rows saying its word, 「4 小时」 that was four hours *ago*, two
// spellings of a latency and money with its mark on either side. Each of
// those was one page formatting a thing its own way; this gate is the
// list of the ways that are gone.
//
// Claims (static: the catalog and `src/`, no browser):
//   1. **a time says its direction in two keys and nowhere else**: no
//      `[gui]` sentence carries 「{n}前」 / 「{n}后」 itself — only
//      `time-ago` and `time-in`, which `ago` / `until` fill;
//   2. **the terminal's own sentences that do carry it** (`[cli]`
//      `vitals-taken`, `git-ahead`, …) are reached from the face only
//      through `words.ts`;
//   3. **the ladder does not leave `words.ts`**: `span` is not exported,
//      and no file imports `span` or `lasted` (a bare number of hours is
//      the ambiguity: an age, a deadline or a duration?);
//   4. **a latency is 「23 ms」**: no catalog sentence words one in 毫秒
//      (`{n} 毫秒`), and the latency keys (`row-rtt`,
//      `roads-ms`) are called from `words.ts` alone (`latency`);
//   5. **连不上 is 不通**: the machine-connection sentences never say 离线,
//      无法获取 or 连不上; the retired keys (`row-offline`, `map-last-seen`,
//      `card-age`) stay retired; `day-unreachable` (the quota page's
//      「无法获取」, which is about a reading, not a machine) is the quota
//      page's alone;
//   6. **the filter and the row are one derivation**: the machines list's
//      `link:` axis picks `connectionKey` and words it `connectionKeyWord`,
//      and the row's state is `connectionKeyWord(connectionKey(d))`, not
//      a second ladder (`views/DevicesList`);
//   7. **money is one formatter**: no file but `words.ts` defines a
//      `money` (or the ring's `ringWord`) of its own — a page waiting on
//      a lane is named in `PENDING` and printed every run (none since
//      UX-53);
//   8. **the direction is not trimmed off** `until` / `ago`'s answer
//      (`untilBare` is the bare two-unit form).
//
// Red first: on 342d4e52 claims 1 to 6 fail — seven [gui] sentences with
// their own 前 / 后, `span` exported and imported by two pages, five 毫秒,
// 离线 / 无法获取 in nine machine sentences, three retired keys in use,
// DevicesList's second ladder (27 findings on the lane's run). Claim 7's
// one offender is the quota page, pending on ux-q2.
//
// Run: `npm run words` (also in `npm test`).
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const gui = fileURLToPath(new URL("..", import.meta.url));
const src = join(gui, "src");
const catalog = join(gui, "..", "..", "crates", "catalog");

let bad = 0;
const fail = (m) => {
  console.error(`words gate: ${m}`);
  bad += 1;
};

/** `[section] key = "value"` lines of a toml — single-line values only,
    which is every sentence this gate reads (the usage block is the one
    multi-line string, and it is help text, not a word on the face). */
function table(file) {
  const out = new Map();
  let sec = "";
  for (const line of readFileSync(join(catalog, file), "utf8").split("\n")) {
    const head = /^\[(\w+)\]\s*$/.exec(line);
    if (head) {
      sec = head[1];
      continue;
    }
    const kv = /^([a-z0-9_-]+) = "(.*)"$/.exec(line);
    if (kv) out.set(`${sec}.${kv[1]}`, kv[2]);
  }
  return out;
}
const zh = table("zh.toml");
const ident = (key) => key.replace(/-/g, "_");

/** Every source file under `src/` but the generated ones, with its text. */
const files = [];
(function walk(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) {
      if (name !== "gen") walk(p);
    } else if (/\.(ts|tsx)$/.test(name)) files.push({ path: relative(gui, p), text: readFileSync(p, "utf8") });
  }
})(src);
const WORDS = "src/words.ts";
/**
 * **Waiting on a lane, by name, with what is left** — never a quiet pass.
 * Each entry is printed on every run; drop it when the lane lands.
 * ⚠️ 墓碑: the quota page's own `money` and `ringWord`, pending on ux-q2
 * until UX-53 put its rows, account page and caption on `money` /
 * `moneyIn` / `agoOf` / `untilBare`.
 */
// UX-53 (ux-q2, 2026-10-08): the quota page took words.ts money / moneyIn,
// agoOf and untilBare; nothing is pending.
const PENDING = new Map();
const words = files.find((f) => f.path === WORDS);
if (!words) fail("probe dead: src/words.ts is not there");
/** Code without its comments, so a tombstone naming a retired key is not
    a use of it. */
const code = (t) => t.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|[^:"'`])\/\/.*$/gm, "$1");
/** Files (but `words.ts`) whose code matches. */
const users = (re, except = [WORDS]) => files.filter((f) => !except.includes(f.path) && re.test(code(f.text))).map((f) => f.path);

// 1. direction in two keys.
const DIRECTED = /\{\d+\}\s?(前|后)/;
const own = [...zh].filter(([k, v]) => k.startsWith("gui.") && DIRECTED.test(v) && k !== "gui.time-ago" && k !== "gui.time-in");
if (own.length) fail(`[gui] sentences that say their own 前 / 后 — they take what ago / until said (words.ts; crates/catalog/zh.toml): ${own.map(([k, v]) => `${k} = ${v}`).join(" | ")}`);
if (zh.get("gui.time-ago") !== "{0}前" || zh.get("gui.time-in") !== "{0}后") fail(`time-ago / time-in read ${zh.get("gui.time-ago")} / ${zh.get("gui.time-in")} — 「{0}前」 and 「{0}后」`);

// 2. the terminal's directed sentences, through words.ts only.
const cliDirected = [...zh].filter(([k, v]) => k.startsWith("cli.") && DIRECTED.test(v)).map(([k]) => ident(k.slice(4)));
for (const key of cliDirected) {
  const at = users(new RegExp(`\\bcli\\.${key}\\b`));
  if (at.length) fail(`cli.${key} carries its own 前 / 后 and is called from ${at.join(", ")} — the face reaches it through words.ts`);
}

// 3. the ladder stays home.
if (words && /export\s+function\s+span\b/.test(words.text)) fail("words.ts exports span — a bare duration leaves the file; pages ask ago, until or lasting");
for (const name of ["span", "lasted"]) {
  const at = users(new RegExp(`import\\s*\\{[^}]*\\b${name}\\b[^}]*\\}\\s*from\\s*["']@/words["']`));
  if (at.length) fail(`${at.join(", ")} import ${name} from words — ago (a moment past), until (a moment ahead) or lasting (a duration)`);
}

// 4. 23 ms.
const msZh = [...zh].filter(([, v]) => /\{\d+\}\s*毫秒/.test(v));
if (msZh.length) fail(`latencies worded in 毫秒 — 「23 ms」 (the owner 2026-10-05): ${msZh.map(([k, v]) => `${k} = ${v}`).join(" | ")}`);
for (const key of ["gui.row_rtt", "cli.roads_ms"]) {
  const at = users(new RegExp(`\\b${key.replace(".", "\\.")}\\b`));
  if (at.length) fail(`${key} called from ${at.join(", ")} — a latency is words.ts latency()`);
}

// 5. 不通.
const MACHINE = [
  "hop.direct", "hop.relay", "hop.quiet", "hop.down", "gui.row-unreachable", "gui.reading-unreachable", "gui.link-last-heard",
  "gui.dial-key-none", "gui.machine-paused", "cli.vitals-never", "gui.identity-pause-warn", "gui.identity-paused-now",
  "gui.net-pause-note", "cli.doctor-peers-paused", "msg.paused", "msg.left-untold",
];
for (const key of MACHINE) {
  const v = zh.get(key);
  if (v === undefined) fail(`probe dead: ${key} is not in zh.toml — the list above names the machine-connection sentences`);
  else if (/离线|无法获取|连不上/.test(v)) fail(`${key} = ${v} — a machine that cannot be reached is 不通 (WORDS-1008)`);
}
if (zh.get("hop.down") !== "不通") fail(`hop.down = ${zh.get("hop.down")} — 不通`);
if (zh.get("cli.vitals-never") !== "尚无读数") fail(`vitals-never = ${zh.get("cli.vitals-never")} — 尚无读数`);
for (const key of ["gui.row-offline", "gui.map-last-seen", "gui.card-age", "gui.conn-relay-ms"]) {
  if (zh.has(key)) fail(`${key} is back in zh.toml — retired by WORDS-1008 (不通 / downWord, agoOf)`);
  const at = users(new RegExp(`\\b${ident(key).replace(".", "\\.")}\\b`), []);
  if (at.length) fail(`${ident(key)} is used in ${at.join(", ")} — retired by WORDS-1008`);
}
const quotaOnly = users(/\bgui\.day_unreachable\b/, [WORDS, "src/views/QuotaLanding.tsx"]);
if (quotaOnly.length) fail(`gui.day_unreachable (「无法获取」) in ${quotaOnly.join(", ")} — a machine is 不通 (hop.down / downWord)`);

// 6. one derivation for the filter and the row.
const devices = files.find((f) => f.path === "src/views/DevicesList.tsx");
if (!devices) fail("probe dead: src/views/DevicesList.tsx is not there");
else {
  const t = code(devices.text);
  if (!/"link:":\s*\(d: DeviceRow\)\s*=>\s*connectionKey\(d\)/.test(t)) fail("DevicesList's link: axis does not pick connectionKey(d) — the filter and the row say one word");
  if (!/label:\s*\(v\)\s*=>\s*connectionKeyWord\(v\)/.test(t)) fail("DevicesList's link: axis does not word its ticks with connectionKeyWord");
  if (!/connectionKeyWord\(\s*key\s*\)/.test(t) || !/const key = connectionKey\(d\)/.test(t)) fail("DevicesList's row state is not connectionKeyWord(connectionKey(d)) — a second ladder is how 离线 and 中继 parted from the filter");
  if (/\bhopWord\(|\bisDown\(/.test(t)) fail("DevicesList words a road itself (hopWord / isDown) — the row's word is connectionKeyWord's");
}

// 7. one money.
const moneyOwn = users(/\bfunction\s+(money|ringWord)\s*\(/).filter((f) => !PENDING.has(f));
if (moneyOwn.length) fail(`${moneyOwn.join(", ")} define a money formatter of their own — words.ts money() (symbol first, 「$9.21」「¥12K」)`);

// 8. the direction is never trimmed off: no page strips 后 / 前 from what
//    `until` / `ago` said (`untilBare` is the bare form).
const trimmed = users(/\b(until|ago|agoOf)\([^)]*\)\s*\.(replace|slice|trim)/);
if (trimmed.length) fail(`${trimmed.join(", ")} cut the direction off what ago / until said — untilBare or lasting`);

for (const [file, what] of PENDING) console.log(`words gate: pending · ${file} · ${what}`);
if (bad) process.exitCode = 1;
else console.log("words gate: 前 and 后 in two keys, the ladder at home, every latency in ms, 不通 for a machine out of reach, one derivation for the filter and the row, one money");
