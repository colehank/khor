// The soft delete gate (item 35): iOS's delete key, as the simulator
// recorded it (`fixtures/ios-delete.json`), replayed into a real xterm with
// `src/lib/term-soft-delete.ts` in front of it, in headless Chrome.
//
// A synthetic event makes no edit of its own, so the replay makes the one
// WebKit would: a `beforeinput` nobody prevented edits the field (a delete
// takes the character, or the word, before the caret; a composition
// replaces its own range), and a keydown that was prevented makes no edit
// at all — which is how xterm's own keydown used to end iOS's repeat.
//
// Asserted, per sequence: the bytes that reach the terminal are exactly
// the expected ones, the placeholder never among them; under the soft
// keyboard the field never runs empty after an edit; with a hardware
// keyboard (`soft` false) xterm's own keydown road is untouched and the
// field is never filled; a pinyin commit is the committed text, once, with
// and without the module.
//
// And item 37: a composition the keyboard goes down in the middle of —
// iOS sends no `compositionend` — is dropped, by the field's `inputmode`
// turning to none or by the screen losing the keyboard: xterm's composition
// view is gone, nothing reaches the terminal, and the next key types clean
// (`src/lib/term-compose-drop.ts`).
import { chromium } from "playwright-core";
import { build } from "esbuild";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const gui = fileURLToPath(new URL("..", import.meta.url));
const fixture = JSON.parse(readFileSync(`${gui}/scripts/fixtures/ios-delete.json`, "utf8"));
let bad = 0;
const fail = (m) => {
  bad++;
  console.error(`soft delete gate: ${m}`);
};
const expand = (s) => s.replace(/(.)×(\d+)/gu, (_, c, n) => c.repeat(Number(n)));
const show = (s) => JSON.stringify(s.replace(/\x7f/g, "⌫"));

const bundle = await build({
  stdin: {
    contents: `
      import { Terminal } from "@xterm/xterm";
      import * as soft from "./src/lib/term-soft-delete.ts";
      import * as drop from "./src/lib/term-compose-drop.ts";
      window.lab = { Terminal, soft, drop };
    `,
    resolveDir: gui,
    loader: "ts",
  },
  bundle: true,
  format: "iife",
  write: false,
  logLevel: "silent",
});
const css = readFileSync(`${gui}/node_modules/@xterm/xterm/css/xterm.css`, "utf8");

const browser = await chromium.launch({ channel: "chrome", headless: true });
const page = await browser.newPage({ viewport: { width: 420, height: 600 } });
page.on("pageerror", (e) => fail(`page error: ${e.message}`));
await page.setContent(`<!doctype html><meta charset=utf-8><style>${css} body{margin:0}</style><div id=screen style="width:400px;height:300px"></div>`);
await page.addScriptTag({ content: bundle.outputFiles[0].text });

const run = (events, softOn, withModule) =>
  page.evaluate(
    async ({ events, softOn, withModule }) => {
      const { Terminal, soft, drop } = window.lab;
      window.undrop?.();
      window.t?.dispose();
      window.sd?.stop();
      const t = new Terminal({ cols: 40, rows: 10 });
      window.t = t;
      t.open(document.getElementById("screen"));
      const field = t.textarea;
      field.value = "";
      const sent = [];
      t.onData((d) => sent.push(d));
      const sd = withModule
        ? soft.softDeletes(field, { soft: () => softOn, send: (k) => t.input(k === "word" ? "\x17" : k === "line" ? "\x15" : k === "forward" ? "\x1b[3~" : "\x7f", true) })
        : null;
      window.sd = sd;
      t.attachCustomKeyEventHandler((ev) => !(sd && sd.key(ev) === false));
      window.kbUp = true;
      window.dropped = 0;
      window.undrop = withModule
        ? drop.dropsComposition(t, {
            keyboardUp: () => window.kbUp,
            onKeyboardChange: (run) => ((window.kbRun = run), () => {}),
            dropped: () => window.dropped++,
          })
        : null;
      field.focus();
      let keyPrevented = false;
      let comp = null; // [start, end) of the composition
      let emptied = 0;
      let filled = false;
      const caret = () => field.selectionStart ?? field.value.length;
      const set = (v, at) => {
        field.value = v;
        field.setSelectionRange(at, at);
      };
      const edit = (inputType, data) => {
        const v = field.value;
        const c = caret();
        if (inputType === "deleteContentBackward") return c > 0 && set(v.slice(0, c - 1) + v.slice(c), c - 1);
        if (inputType === "deleteWordBackward") {
          let i = c;
          while (i > 0 && /\s/u.test(v[i - 1])) i--;
          while (i > 0 && !/\s/u.test(v[i - 1])) i--;
          return set(v.slice(0, i) + v.slice(c), i);
        }
        if (inputType === "insertText") return set(v.slice(0, c) + data + v.slice(c), c + data.length);
        if (inputType === "insertCompositionText") {
          const [s, e] = comp ?? [c, c];
          set(v.slice(0, s) + data + v.slice(e), s + data.length);
          comp = [s, s + data.length];
          return;
        }
        if (inputType === "deleteCompositionText" && comp) {
          set(v.slice(0, comp[0]) + v.slice(comp[1]), comp[0]);
          comp = [comp[0], comp[0]];
          return;
        }
        if (inputType === "insertFromComposition") {
          const s = comp ? comp[0] : c;
          set(v.slice(0, s) + data + v.slice(s), s + data.length);
        }
      };
      const key = (type, k, code, composing) => {
        const ev = new KeyboardEvent(type, { key: k, bubbles: true, cancelable: true, isComposing: !!composing });
        Object.defineProperty(ev, "keyCode", { get: () => code });
        Object.defineProperty(ev, "which", { get: () => code });
        field.dispatchEvent(ev);
        if (type === "keydown") keyPrevented = ev.defaultPrevented;
        if (type === "keyup") keyPrevented = false;
      };
      const input = (inputType, data, composing) => {
        const b = new InputEvent("beforeinput", { inputType, data: data ?? null, isComposing: !!composing, bubbles: true, cancelable: true });
        field.dispatchEvent(b);
        if (b.defaultPrevented || keyPrevented) return;
        edit(inputType, data ?? "");
        field.dispatchEvent(new InputEvent("input", { inputType, data: data ?? null, isComposing: !!composing, bubbles: true }));
        if (inputType.startsWith("delete") && !composing) {
          if (field.value.length === 0) emptied++;
          if (field.value.length > 0) filled = true;
        }
      };
      const tick = () => new Promise((z) => setTimeout(z, 2));
      for (const e of events) {
        if (e.type === "keydown" || e.type === "keyup") key(e.type, e.key, e.code, e.composing);
        else if (e.type === "beforeinput") input(e.inputType, e.data, e.composing);
        else if (e.type === "input") {
          // Dispatched together with its beforeinput, as the edit's second half.
        } else if (e.type.startsWith("composition")) {
          if (e.type === "compositionstart") comp = [caret(), caret()];
          field.dispatchEvent(new CompositionEvent(e.type, { data: e.data ?? "", bubbles: true }));
          if (e.type === "compositionend") comp = null;
        } else if (e.type === "inputmode-none") {
          field.setAttribute("inputmode", "none");
          await new Promise((z) => setTimeout(z, 20));
        } else if (e.type === "keyboard-down") {
          window.kbUp = false;
          window.kbRun?.();
        } else if (e.type === "repeat") {
          for (let i = 0; i < e.times; i++) {
            input(e.inputType);
            await tick();
          }
        } else if (e.type === "repeat-keyed") {
          for (let i = 0; i < e.times; i++) {
            key("keydown", "Backspace", 8);
            input(e.inputType);
            await tick();
          }
        }
        await tick();
      }
      await new Promise((z) => setTimeout(z, 50));
      const helper = t._core?._compositionHelper;
      return { sent: sent.join(""), emptied, filled, value: field.value, dropped: window.dropped, composing: !!helper?._isComposing, viewActive: !!helper?._compositionView?.classList.contains("active") };
    },
    { events, softOn, withModule },
  );

const NBSP = " ";
for (const s of fixture.sequences) {
  const want = expand(s.sends);
  const got = await run(s.events, true, true);
  if (got.sent !== want) fail(`${s.name} (${s.source}), soft keyboard: sent ${show(got.sent)}, want ${show(want)}`);
  if (got.sent.includes(NBSP)) fail(`${s.name}: the placeholder reached the terminal`);
  if (got.emptied > 0) fail(`${s.name}: the field ran empty ${got.emptied} times under a held delete`);
  console.log(`soft delete gate: ${s.name} (${s.source}) → ${show(got.sent)}`);
}
// A hardware keyboard: xterm's own keydown, one DEL a press, nothing filled.
{
  const s = fixture.sequences[0];
  const got = await run(s.events, false, true);
  if (got.sent !== "\x7f\x7f") fail(`hardware keyboard: sent ${show(got.sent)}, want ⌫⌫`);
  if (got.value.includes(NBSP)) fail("hardware keyboard: the field was filled");
  console.log(`soft delete gate: hardware keyboard → ${show(got.sent)}, field ${JSON.stringify(got.value)}`);
}
// Without the module: the commit is the same, and a held delete is one key —
// the owner's report, in the gate.
{
  const pinyin = fixture.sequences.find((s) => s.name.startsWith("pinyin:"));
  const plain = await run(pinyin.events, true, false);
  if (plain.sent !== "你\x7f") fail(`pinyin without the module: sent ${show(plain.sent)}`);
  const held = fixture.sequences.find((s) => s.name.startsWith("held delete, 40 repeats as edits"));
  const before = await run(held.events, true, false);
  console.log(`soft delete gate: without the module, a held delete sends ${show(before.sent)} (xterm's keydown ends the repeat)`);
  if (before.sent !== "\x7f") fail(`without the module a held delete sent ${show(before.sent)}: the replay no longer shows the bug`);
}

// Item 37: the keyboard goes down in the middle of a composition.
{
  const half = [
    { type: "keydown", key: "n", code: 229 },
    { type: "compositionstart", data: "" },
    { type: "compositionupdate", data: "n" },
    { type: "beforeinput", inputType: "insertCompositionText", data: "n", composing: true },
    { type: "keyup", key: "n", code: 78, composing: true },
    { type: "keydown", key: "i", code: 229, composing: true },
    { type: "compositionupdate", data: "ni" },
    { type: "beforeinput", inputType: "insertCompositionText", data: "ni", composing: true },
    { type: "keyup", key: "i", code: 73, composing: true },
  ];
  const after = [
    { type: "keydown", key: "a", code: 65 },
    { type: "beforeinput", inputType: "insertText", data: "a" },
    { type: "keyup", key: "a", code: 65 },
  ];
  for (const [how, down] of [["inputmode none", { type: "inputmode-none" }], ["the screen lost the keyboard", { type: "keyboard-down" }]]) {
    const got = await run([...half, down, ...after], true, true);
    if (got.dropped !== 1) fail(`37, ${how}: the composition was dropped ${got.dropped} times`);
    if (got.viewActive || got.composing) fail(`37, ${how}: xterm still composing (view ${got.viewActive}, state ${got.composing})`);
    if (got.sent !== "a") fail(`37, ${how}: sent ${show(got.sent)}, want "a": nothing of the pinyin, then the next key`);
    console.log(`soft delete gate: 37, keyboard down mid-composition (${how}) → dropped, then sent ${show(got.sent)}`);
  }
  const still = await run([...half, { type: "keyboard-down" }], true, false);
  const later = await run([...half, { type: "keyboard-down" }, ...after], true, false);
  console.log(`soft delete gate: 37, without the module xterm is still composing (${still.composing}), and the next key sends ${show(later.sent)}`);
  if (!still.composing) fail("37: without the module the replay no longer shows the stuck composition");
}

await browser.close();
if (bad) {
  console.error(`soft delete gate: ${bad} failure(s)`);
  process.exit(1);
}
console.log("soft delete gate: the soft keyboard's delete repeats, and nothing else changed");
