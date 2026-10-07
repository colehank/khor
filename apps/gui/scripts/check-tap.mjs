// The tap gate (item 43): which cells a tap raises the keyboard from, in
// `auto` (`src/lib/term-marks.ts` `inInput`), run in Node against a real
// xterm buffer. A program's input box (Claude Code's `─` edges around the
// `>` line) counts with two rows beyond each edge and no more — not down
// to the screen's bottom; a bare cursor row counts with one row either
// side; a shell at its prompt without marks counts its row and the blank
// below it, as item 28 ruled.
//
// Item 49: what a tap asks of the keyboard (`src/lib/term-tap.ts`
// `tapAsks`) — in 自动 the input area raises and anywhere else lowers
// (the 10-05 「点别处不动」 is overturned); 屏幕 raises anywhere and never
// lowers; 按钮 leaves the keyboard to its switch.
//
// Item 74: with no cursor on screen (`?25l`, as `top` does) a tap in 自动
// raises what is down and lowers what is up (`toggle`), wherever it lands;
// whether the cursor is shown is read as xterm parses DECTCEM
// (`watchesCursor`), and a reset (RIS, DECSTR) shows it again.
import { build } from "esbuild";
import { fileURLToPath } from "node:url";

const gui = fileURLToPath(new URL("..", import.meta.url));
const alias = {
  name: "alias",
  setup(b) {
    b.onResolve({ filter: /^@\// }, (a) => b.resolve(`./src/${a.path.slice(2)}`, { resolveDir: gui, kind: a.kind }));
  },
};
const out = await build({
  stdin: {
    contents: `import { Terminal } from "@xterm/xterm"; import { inInput } from "@/lib/term-marks"; import { tapAsks, watchesCursor } from "@/lib/term-tap"; globalThis.lab = { Terminal, inInput, tapAsks, watchesCursor };`,
    resolveDir: gui,
    loader: "ts",
  },
  bundle: true,
  format: "esm",
  platform: "node",
  write: false,
  logLevel: "error",
  define: { "import.meta.env.DEV": "false" },
  plugins: [alias],
});
await import(`data:text/javascript;base64,${Buffer.from(out.outputFiles[0].text).toString("base64")}`);
const { Terminal, inInput, tapAsks, watchesCursor } = globalThis.lab;

let bad = 0;
const fail = (m) => {
  bad++;
  console.error(`tap gate: ${m}`);
};
/** A program's rows, no shell marks. */
const unmarked = { current: () => null, marked: () => false };
const ROWS = 24;

/** A screen of `rows` (top down, the rest blank), the cursor on row `y`;
    which rows a tap raises from. */
async function raises(rows, y, alternate = false) {
  const t = new Terminal({ cols: 40, rows: ROWS, allowProposedApi: true });
  const text = (alternate ? "\x1b[?1049h" : "") + rows.map((r, i) => `\x1b[${i + 1};1H${r}`).join("");
  await new Promise((r) => t.write(text, r));
  const top = t.buffer.active.baseY;
  const on = [];
  for (let line = 0; line < ROWS; line++) if (inInput(t, unmarked, top + line, 3, { x: 2, y })) on.push(line);
  t.dispose();
  return on;
}
const same = (a, b) => a.length === b.length && a.every((x, i) => x === b[i]);
const range = (a, b) => Array.from({ length: b - a + 1 }, (_, i) => a + i);
const check = async (name, rows, y, want, alternate) => {
  const got = await raises(rows, y, alternate);
  console.log(`tap gate: ${name}: rows ${JSON.stringify(got)}`);
  if (!same(got, want)) fail(`${name}: a tap raises from rows ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
};

// Claude Code 2.1: output, a three-row box (`─`, `>` with the cursor,
// `─`), the hint line, blank below. The box and two rows beyond each
// edge; the output two rows up and the blank under the hint do not.
const edge = "─".repeat(40);
await check(
  "Claude Code's box",
  ["Claude: here is the plan.", "", "1. read the file", "2. edit it", "3. run the tests", "", edge, "> ", edge, "  ? for shortcuts"],
  7,
  range(4, 10),
);
// The same box with nothing under it: still not down to the screen's bottom.
await check("the box with nothing below", ["output", "", "", "", "", "", edge, "> ", edge], 7, range(4, 10));
// A bare cursor row in a full-screen program (an editor): the row and one
// either side.
await check(
  "a bare cursor row",
  Array.from({ length: ROWS }, (_, i) => `line ${i} of the file`),
  10,
  range(9, 11),
  true,
);
// A shell at its prompt without marks: its row, one above, and the blank
// below — a fresh shell's prompt at the top with the screen blank under it.
await check("a shell at its prompt", ["Last login: today", "", "$ "], 2, range(1, ROWS - 1));
// The same under tmux (alternate screen, a status line on the last row).
const tmux = ["Last login: today", "", "$ "];
tmux[ROWS - 1] = "[khor] 0:zsh*";
await check("a shell at its prompt under tmux", tmux.map((r) => r ?? ""), 2, range(1, ROWS - 1), true);
// Output below the prompt row (a command running): the row and one either side only.
await check("a shell mid-command", ["$ sleep 10", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "waiting"], 0, range(0, 1));

// Item 49: the mode and the input area decide; the pane raises only what
// is down and lowers only what is up.
for (const [wake, onInput, want] of [
  ["auto", true, "raise"],
  ["auto", false, "lower"],
  ["screen", true, "raise"],
  ["screen", false, "raise"],
  ["button", true, null],
  ["button", false, null],
]) {
  const got = tapAsks(wake, onInput);
  if (got !== want) fail(`49: a tap in ${wake} ${onInput ? "on" : "off"} the input area asks ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
}
console.log("tap gate: 49: auto raises on the input area and lowers off it; screen raises anywhere; button asks nothing");

// Item 74: with no cursor on screen, auto toggles wherever the tap lands;
// the other modes are as they were.
for (const [wake, want] of [
  ["auto", "toggle"],
  ["screen", "raise"],
  ["button", null],
]) {
  for (const onInput of [true, false]) {
    const got = tapAsks(wake, onInput, true);
    if (got !== want) fail(`74: with no cursor a tap in ${wake} ${onInput ? "on" : "off"} the input area asks ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
  }
}
// …and the cursor is read as xterm parses DECTCEM, back on a reset.
{
  const t = new Terminal({ cols: 40, rows: ROWS, allowProposedApi: true });
  const seen = watchesCursor(t);
  const after = async (bytes) => {
    await new Promise((r) => t.write(bytes, r));
    return seen.shown();
  };
  const steps = [
    ["", true],
    ["\x1b[?1049h\x1b[?25l", false],
    ["\x1b[?25h", true],
    ["\x1b[?1;25l", false],
    ["\x1bc", true],
    ["\x1b[?25l\x1b[!p", true],
  ];
  const got = [];
  for (const [bytes, want] of steps) {
    const now = await after(bytes);
    got.push(now);
    if (now !== want) fail(`74: after ${JSON.stringify(bytes)} the cursor reads ${now ? "shown" : "hidden"}, want ${want ? "shown" : "hidden"}`);
  }
  seen.dispose();
  t.dispose();
  console.log(`tap gate: 74: no cursor → auto toggles; DECTCEM read as ${JSON.stringify(got)}`);
}

if (bad) process.exit(1);
console.log("tap gate: a tap raises from the box and two rows beyond it, from a bare cursor row and one beyond it, and from a shell prompt's blank below; off the input area in auto it lowers");
