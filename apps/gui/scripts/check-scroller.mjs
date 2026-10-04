// The scroller gate (items 29 and 30): the terminal's history on the page's
// own scroller (`src/lib/term-scroller.ts`), driven in headless Chrome
// against a real xterm, in the same DOM the pane builds.
//
//   29. **A resize is not a scroll.** At the bottom with 200 lines of
//       history, the box shrinks the way a rising keyboard shrinks it (in
//       steps, each refitted); afterwards the viewport is still at the
//       bottom (`viewportY === baseY`) and the scroller is at its end.
//   30. **Output during a gesture does not move the scroller.** A program
//       redraws a 40-row frame at 10 Hz on a 24-row terminal (Claude Code's
//       Ink on a phone) while a finger drags up through the history: until
//       the lift, every `scrollTop` the scroller takes is one the finger
//       set; on settle the code places it once.
import { chromium } from "playwright-core";
import { build } from "esbuild";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const gui = fileURLToPath(new URL("..", import.meta.url));
let bad = 0;
const fail = (m) => {
  bad++;
  console.error(`scroller gate: ${m}`);
};

const bundle = await build({
  stdin: {
    contents: `
      import { Terminal } from "@xterm/xterm";
      import { FitAddon } from "@xterm/addon-fit";
      import * as scroller from "./src/lib/term-scroller.ts";
      window.lab = { Terminal, FitAddon, scroller };
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
const page = await browser.newPage({ viewport: { width: 420, height: 800 }, deviceScaleFactor: 1 });
page.on("pageerror", (e) => fail(`page error: ${e.message}`));
await page.setContent(`<!doctype html><meta charset=utf-8><style>${css}
  body{margin:0;background:#000}
  #box{position:relative;width:400px;height:600px;overflow:hidden}
  #scroller{height:100%;width:100%;overflow-x:hidden;overflow-y:auto}
  #stick{position:sticky;top:0;height:100%;width:100%}
  #screen{height:100%;width:100%}
  </style><div id=box><div id=scroller><div id=stick><div id=screen></div></div><div id=room></div></div></div>`);
await page.addScriptTag({ content: bundle.outputFiles[0].text });

const fresh = (scrollback = 2000) =>
  page.evaluate(async (scrollback) => {
    const { Terminal, FitAddon, scroller } = window.lab;
    window.t?.dispose();
    window.native?.stop();
    document.querySelector("#room").style.height = "0px";
    const term = new Terminal({ fontFamily: "monospace", fontSize: 14, lineHeight: 1, scrollback });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(document.querySelector("#screen"));
    fit.fit();
    window.t = term;
    window.fit = fit;
    window.resizes = 0;
    term.onResize(() => window.resizes++);
    window.native = scroller.scrollsNatively(term, document.querySelector("#scroller"), document.querySelector("#room"), document.querySelector("#screen"));
    return { cols: term.cols, rows: term.rows };
  }, scrollback);
const frames = (n) => page.evaluate((n) => new Promise((r) => { let i = 0; const f = () => (++i >= n ? r() : requestAnimationFrame(f)); requestAnimationFrame(f); }), n);

// ── 29 ──
await fresh();
await page.evaluate(() => new Promise((r) => { let s = ""; for (let i = 1; i <= 200; i++) s += `line ${i}\r\n`; window.t.write(s, r); }));
await frames(4);
const before = await page.evaluate(() => ({ y: window.t.buffer.active.viewportY, base: window.t.buffer.active.baseY }));
if (before.y !== before.base) fail(`probe dead: not at the bottom before the resize (${JSON.stringify(before)})`);
// The keyboard rising: the box shrinks in five steps, each fitted, as the
// pane's resize observer does.
for (const h of [560, 500, 440, 380, 350]) {
  await page.evaluate((h) => { document.querySelector("#box").style.height = `${h}px`; }, h);
  await frames(1);
  await page.evaluate(() => window.fit.fit());
  await frames(1);
}
await frames(6);
const after29 = await page.evaluate(() => {
  const b = window.t.buffer.active;
  const s = document.querySelector("#scroller");
  return { y: b.viewportY, base: b.baseY, gap: Math.round(s.scrollHeight - s.clientHeight - s.scrollTop), rows: window.t.rows };
});
console.log(`scroller gate: 29 after the box shrank: ${JSON.stringify(after29)}`);
if (after29.y !== after29.base) fail(`29: a resize at the bottom left the viewport ${after29.base - after29.y} lines up`);
if (Math.abs(after29.gap) > 1) fail(`29: the scroller is ${after29.gap} px from its end after the resize`);

// ── 30 ── A short scrollback, so the program's frames also trim the
// history under the finger, as a long session's do.
await fresh(400);
await page.evaluate(async () => {
  // A 24-row screen with history, then Ink-style frames taller than it.
  await new Promise((r) => { let s = ""; for (let i = 1; i <= 300; i++) s += `old ${i}\r\n`; window.t.write(s, r); });
});
await page.evaluate((h) => { document.querySelector("#box").style.height = `${h}px`; window.fit.fit(); }, Math.round((600 / 42) * 24));
await frames(4);
const run = await page.evaluate(async () => {
  const sc = document.querySelector("#scroller");
  const t = window.t;
  const mine = new Set();
  const seen = [];
  let lifted = false;
  let afterLift = 0;
  sc.addEventListener("scroll", () => {
    const top = Math.round(sc.scrollTop);
    if (mine.has(top)) return;
    if (lifted) afterLift++;
    else seen.push(top);
  });
  // The program: a 40-row frame at 10 Hz, redrawn the way Ink does it.
  let n = 0;
  const frame = () => {
    n++;
    let s = "\x1b[?25l";
    if (n > 1) s += "\x1b[40A\r";
    for (let r = 0; r < 40; r++) s += `frame ${n} row ${r}\x1b[K\r\n`;
    t.write(s + "\x1b[?25h");
  };
  const timer = setInterval(frame, 100);
  // The finger: down, then up through the history in 30 steps over 1.5 s.
  const touch = (type) => sc.dispatchEvent(new TouchEvent(type, { bubbles: true, touches: type === "touchend" ? [] : [new Touch({ identifier: 1, target: sc, clientY: 100 })] }));
  touch("touchstart");
  const from = sc.scrollTop;
  for (let i = 1; i <= 30; i++) {
    await new Promise((r) => setTimeout(r, 50));
    const top = Math.round(from - i * 20);
    mine.add(top);
    sc.scrollTop = top;
  }
  touch("touchend");
  lifted = true;
  clearInterval(timer);
  await new Promise((r) => setTimeout(r, 600));
  return { duringDrag: seen.length, examples: seen.slice(0, 5), afterLift };
});
console.log(`scroller gate: 30 scroller moves not the finger's during the drag: ${run.duringDrag} ${JSON.stringify(run.examples)}; after the lift: ${run.afterLift}`);
if (run.duringDrag > 0) fail(`30: output moved the scroller ${run.duringDrag} times while the finger held it`);
if (run.afterLift > 2) fail(`30: the settle placed the scroller ${run.afterLift} times, not once`);

await browser.close();
if (bad) process.exit(1);
console.log("scroller gate: a resize keeps the bottom, and output never moves the scroller under a finger");
