// The extended keys gate (item 39): what a modified key sends once the
// program, or tmux on its behalf, asked for extended keys
// (`src/lib/term-kitty.ts`). Run in Node against a real xterm: the
// requests go in through xterm's parser as a program's output would, and
// the keys through the pane's own encoder.
//
// Asserted:
// - nothing asked: every key is left to xterm, except Shift+Enter, which
//   is `ESC CR` (item 59: Meta+Enter, a newline in Claude Code, codex and
//   zsh, and what tmux with `extended-keys off` passes through);
// - xterm's modifyOtherKeys (`CSI > 4 ; 2 m`, what tmux asks of an outer
//   terminal with `extkeys`, measured from tmux 3.7b): Shift+Enter is
//   `CSI 27 ; 2 ; 13 ~`, Ctrl and Alt combinations too; level 1 leaves the
//   combinations with a control character of their own; `CSI > 4 m` and
//   RIS end it; modifyCursorKeys (`CSI > 2 ; 1 m`) is not it;
// - the kitty flags (`CSI > 1 u`) win when both were asked: `CSI 13 ; 2 u`;
// - the key row's 换行 cap (item 40) sends what Shift+Enter sends in every
//   one of those states, and repeats while held;
// - Ctrl and Alt (item 69): one press waits for one key, a double press
//   locks for every key until pressed again, a late second press lets go.
// The bytes crossing a real tmux unchanged are the host's test,
// `crates/node/tests/tmux_extkeys.rs`.
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
    contents: `import { Terminal } from "@xterm/xterm"; import * as kitty from "@/lib/term-kitty"; import { KEY_BY_NAME, pressLatch, afterKey, typed, LOCK_TAP_MS } from "@/lib/term-keys"; globalThis.lab = { Terminal, kitty, KEY_BY_NAME, pressLatch, afterKey, typed, LOCK_TAP_MS };`,
    resolveDir: gui,
    loader: "ts",
  },
  bundle: true,
  format: "esm",
  platform: "node",
  write: false,
  logLevel: "error",
  plugins: [alias],
});
await import(`data:text/javascript;base64,${Buffer.from(out.outputFiles[0].text).toString("base64")}`);
const { Terminal, kitty, KEY_BY_NAME, pressLatch, afterKey, typed, LOCK_TAP_MS } = globalThis.lab;
const newline = KEY_BY_NAME.get("newline");

let bad = 0;
const fail = (m) => {
  bad++;
  console.error(`keys gate: ${m}`);
};
const show = (s) => (s === null ? "xterm's own" : JSON.stringify(s));

/** A keydown as the browser gives it. */
const key = (k, code, mods = {}) => ({
  type: "keydown",
  key: k,
  code,
  shiftKey: !!mods.shift,
  altKey: !!mods.alt,
  ctrlKey: !!mods.ctrl,
  metaKey: !!mods.meta,
  repeat: false,
  isComposing: false,
  keyCode: 0,
});
const SHIFT_ENTER = key("Enter", "Enter", { shift: true });
const ENTER = key("Enter", "Enter");
const CTRL_ENTER = key("Enter", "Enter", { ctrl: true });
const SHIFT_TAB = key("Tab", "Tab", { shift: true });
const CTRL_A = key("a", "KeyA", { ctrl: true });
const CTRL_SHIFT_A = key("A", "KeyA", { ctrl: true, shift: true });
const SHIFT_A = key("A", "KeyA", { shift: true });
const CMD_ENTER = key("Enter", "Enter", { meta: true });

/** A fresh pane that has read `stream`; what each key sends. */
async function after(stream, keys) {
  const t = new Terminal({ cols: 40, rows: 10, allowProposedApi: true });
  const answers = [];
  kitty.kittyProtocol(t, (b) => answers.push(b));
  await new Promise((r) => t.write(stream, r));
  const sent = keys.map((ev) => kitty.kittyKey(t, ev, false));
  const cap = newline?.send?.(t, { ctrl: false, alt: false });
  t.dispose();
  return { sent, cap };
}

const E = "\x1b";
const cases = [
  ["nothing asked", "", [[SHIFT_ENTER, `${E}\r`], [ENTER, null], [CTRL_ENTER, null], [CTRL_A, null], [CMD_ENTER, null]]],
  [
    "modifyOtherKeys 2 (tmux with extkeys)",
    `${E}[>4;2m`,
    [
      [SHIFT_ENTER, `${E}[27;2;13~`],
      [ENTER, null],
      [CTRL_ENTER, `${E}[27;5;13~`],
      [SHIFT_TAB, `${E}[27;2;9~`],
      [CTRL_A, `${E}[27;5;97~`],
      [CTRL_SHIFT_A, `${E}[27;6;65~`],
      [SHIFT_A, null],
      [CMD_ENTER, null],
    ],
  ],
  [
    "modifyOtherKeys 1",
    `${E}[>4;1m`,
    [
      [SHIFT_ENTER, `${E}[27;2;13~`],
      [CTRL_A, null],
      [CTRL_SHIFT_A, `${E}[27;6;65~`],
    ],
  ],
  ["modifyOtherKeys reset", `${E}[>4;2m${E}[>4m`, [[SHIFT_ENTER, `${E}\r`]]],
  ["modifyOtherKeys, then RIS", `${E}[>4;2m${E}c`, [[SHIFT_ENTER, `${E}\r`]]],
  ["modifyCursorKeys is not it", `${E}[>2;1m`, [[SHIFT_ENTER, `${E}\r`]]],
  ["kitty flags", `${E}[>1u`, [[SHIFT_ENTER, `${E}[13;2u`], [ENTER, null]]],
  ["kitty flags and modifyOtherKeys: kitty wins", `${E}[>4;2m${E}[>1u`, [[SHIFT_ENTER, `${E}[13;2u`]]],
  ["kitty popped, modifyOtherKeys left", `${E}[>4;2m${E}[>1u${E}[<u`, [[SHIFT_ENTER, `${E}[27;2;13~`]]],
];

let checked = 0;
for (const [name, stream, expect] of cases) {
  const { sent, cap } = await after(stream, expect.map(([ev]) => ev));
  // The 换行 cap: what this state's Shift+Enter sends.
  const wantCap = expect.find(([ev]) => ev === SHIFT_ENTER)?.[1];
  if (wantCap !== undefined) {
    checked++;
    if (cap !== wantCap) fail(`${name}: the 换行 cap sent ${show(cap ?? null)}, want ${show(wantCap)}`);
  }
  expect.forEach(([ev, want], i) => {
    checked++;
    const label = `${ev.shiftKey ? "⇧" : ""}${ev.ctrlKey ? "⌃" : ""}${ev.altKey ? "⌥" : ""}${ev.metaKey ? "⌘" : ""}${ev.key}`;
    if (sent[i] !== want) fail(`${name}: ${label} sent ${show(sent[i])}, want ${show(want)}`);
  });
}

checked++;
if (!newline) fail("no 换行 cap in the key set");
else if (newline.repeats !== true) fail("the 换行 cap does not repeat while held");

// **Ctrl and Alt lock on a double press** (item 69). Each step: a press
// on a latch at a time, or a key typed under the latches.
{
  const rest = { ctrl: false, alt: false };
  const said = [];
  const run = (steps) => {
    let m = rest;
    const out = [];
    for (const [what, at] of steps) {
      if (what === "ctrl" || what === "alt") m = pressLatch(m, what, at);
      else {
        out.push(typed(what, m));
        if (m.ctrl || m.alt) m = afterKey(m);
      }
    }
    return { out: out.join(""), m };
  };
  const once = run([["ctrl", 1000], ["c", 1100], ["c", 1200]]);
  said.push(JSON.stringify(once.out));
  if (once.out !== "\x03c") fail(`one press is one key: ${JSON.stringify(once.out)}`);
  const locked = run([["ctrl", 2000], ["ctrl", 2000 + LOCK_TAP_MS - 50], ["c", 2500], ["d", 2600], ["l", 2700]]);
  said.push(JSON.stringify(locked.out));
  if (locked.out !== "\x03\x04\x0c") fail(`a double press locks for every key: ${JSON.stringify(locked.out)}`);
  if (!locked.m.ctrl || !locked.m.lock?.ctrl) fail("locked Ctrl let go after a key");
  const freed = run([["ctrl", 3000], ["ctrl", 3100], ["c", 3200], ["ctrl", 4000], ["c", 4100]]);
  said.push(JSON.stringify(freed.out));
  if (freed.out !== "\x03c" || freed.m.ctrl || freed.m.lock?.ctrl) fail(`a press on a locked Ctrl lets go: ${JSON.stringify(freed.out)}`);
  const late = run([["alt", 5000], ["alt", 5000 + LOCK_TAP_MS + 50], ["x", 6000]]);
  said.push(JSON.stringify(late.out));
  if (late.out !== "x") fail(`a late second press lets go, nothing sent: ${JSON.stringify(late.out)}`);
  const both = run([["alt", 7000], ["alt", 7100], ["ctrl", 8000], ["c", 8100], ["c", 8200]]);
  said.push(JSON.stringify(both.out));
  if (both.out !== "\x1b\x03\x1bc") fail(`a locked Alt under a one-shot Ctrl: ${JSON.stringify(both.out)}`);
  console.log(`keys gate: latches ${said.join(" ")}`);
}
if (bad) {
  console.error(`keys gate: ${bad} of ${checked} wrong`);
  process.exit(1);
}
console.log(`keys gate: ${checked} keys right across ${cases.length} terminal states`);
