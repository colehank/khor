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
//   36. **A program's wheel by the scroller's travel.** Where the program
//       holds the mouse (Claude Code in tmux), under a finger the scroller
//       keeps a range of its own and every row it travels is one notch:
//       300 px of travel at 16 px rows is 18 notches (it was 5: a wheel per
//       touch move, folded by xterm), the place goes back to the middle
//       once it rests, and nothing is handed on while it does.
//   32. **The folded deck's band** (`src/lib/term-fold-band.ts`). Folded,
//       the band above the grid holds the two lines just above the
//       source's viewport — the live buffer, then a tmux history view's
//       when one is given — at the bottom and scrolled up; it takes the
//       grid's part-row shift; a program on its alternate screen with no
//       history leaves it empty, and so does unfolding. No resize.
//   42. **The swipe that lowers the keyboard is the finger reaching the
//       keyboard's top edge** (`src/lib/term-swipe.ts`): a drag more down
//       than across lowers it the moment it crosses the edge, however slow
//       or paused, once; one that stops short, goes more across, starts on
//       the bar, or finds no keyboard up lowers nothing.
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
      import { drawsFoldBand } from "./src/lib/term-fold-band.ts";
      import { swipeLowers } from "./src/lib/term-swipe.ts";
      window.lab = { Terminal, FitAddon, scroller, drawsFoldBand, swipeLowers };
    `,
    resolveDir: gui,
    loader: "ts",
  },
  bundle: true,
  format: "iife",
  write: false,
  logLevel: "silent",
  define: { "import.meta.env.DEV": "false" },
  plugins: [{
    name: "alias",
    setup(b) {
      b.onResolve({ filter: /^@\// }, (a) => b.resolve(`./src/${a.path.slice(2)}`, { resolveDir: gui, kind: a.kind }));
    },
  }],
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
  #band{position:absolute;left:0;right:0;top:0;overflow:hidden;display:none}
  #box[data-folded] #band{display:block}
  </style><div id=box><div id=scroller><div id=stick><div id=screen></div><div id=band></div></div><div id=room></div></div></div>`);
await page.addScriptTag({ content: bundle.outputFiles[0].text });

const fresh = (scrollback = 2000, wheel = false) =>
  page.evaluate(async ({ scrollback, wheel }) => {
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
    window.notches = [];
    window.native = scroller.scrollsNatively(term, document.querySelector("#scroller"), document.querySelector("#room"), document.querySelector("#screen"), wheel ? { wheel: (n) => window.notches.push(n) } : {});
    return { cols: term.cols, rows: term.rows };
  }, { scrollback, wheel });
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

// ── 36 ──
await page.evaluate(() => { document.querySelector("#box").style.height = "600px"; });
await fresh(2000, true);
await page.evaluate(() => new Promise((r) => window.t.write("\x1b[?1002h\x1b[?1006h", r)));
await frames(4);
const wheel = await page.evaluate(async () => {
  const sc = document.querySelector("#scroller");
  const row = document.querySelector(".xterm-screen").getBoundingClientRect().height / window.t.rows;
  const range = Math.round(sc.scrollHeight - sc.clientHeight);
  const mid = Math.round(sc.scrollTop);
  const touch = (type) => sc.dispatchEvent(new TouchEvent(type, { bubbles: true, touches: type === "touchend" ? [] : [new Touch({ identifier: 1, target: sc, clientY: 100 })] }));
  touch("touchstart");
  for (let i = 1; i <= 15; i++) {
    await new Promise((r) => setTimeout(r, 20));
    sc.scrollTop = mid - i * 20;
  }
  touch("touchend");
  await new Promise((r) => setTimeout(r, 30));
  const atLift = window.notches.reduce((a, b) => a + b, 0);
  await new Promise((r) => setTimeout(r, 600));
  return { row, range, mid, atLift, settled: window.notches.reduce((a, b) => a + b, 0), back: Math.round(sc.scrollTop) };
});
console.log(`scroller gate: 36 wheel by travel: ${JSON.stringify(wheel)}`);
const want = -Math.trunc(300 / wheel.row);
if (wheel.range < 10000) fail(`36: the program's scroller has no range of its own (${wheel.range})`);
if (wheel.atLift !== want) fail(`36: 300 px of travel handed ${wheel.atLift} notches, want ${want}`);
if (wheel.settled !== wheel.atLift) fail(`36: setting the place back handed ${wheel.settled - wheel.atLift} more notches`);

// ── 32 ──
await fresh();
await page.evaluate(() => new Promise((r) => { let s = ""; for (let i = 1; i <= 200; i++) s += `\x1b[3${i % 8}mline ${i}\x1b[0m end\r\n`; window.t.write(s, r); }));
await frames(4);
const band = await page.evaluate(async () => {
  const box = document.querySelector("#box");
  const el = document.querySelector("#band");
  const screen = document.querySelector("#screen");
  const t = window.t;
  const frames = (n) => new Promise((r) => { let i = 0; const f = () => (++i >= n ? r() : requestAnimationFrame(f)); requestAnimationFrame(f); });
  let history = null;
  const b = window.lab.drawsFoldBand({ band: el, root: box, live: t, history: () => history, screens: [screen] });
  const row = document.querySelector(".xterm-screen").getBoundingClientRect().height / t.rows;
  const look = (src) => {
    const rows = [...el.children].map((x) => x.textContent.replace(/\s+$/, ""));
    const buf = src?.buffer.active;
    const want = buf ? rows.map((_, i) => buf.getLine(buf.viewportY - rows.length + i)?.translateToString(true)) : [];
    return { n: Number(el.dataset.rows), rows, same: rows.length > 0 && rows.every((x, i) => x === want[i]), coloured: el.querySelectorAll("span[style*='color']").length };
  };
  const out = {};
  // The fold, as UX-13 makes it: the box grows by the band, the grid moves down.
  box.style.height = `${600 + 2 * row}px`;
  el.style.height = `${2 * row}px`;
  document.querySelector("#stick").style.paddingTop = `${2 * row}px`;
  box.dataset.folded = "true";
  await frames(4);
  out.bottom = look(t);
  t.scrollLines(-30);
  await frames(3);
  out.up = look(t);
  screen.style.transform = "translateY(5px)";
  await frames(1);
  out.shift = el.style.transform;
  screen.style.transform = "";
  await frames(1);
  out.unshift = el.style.transform;
  // A tmux pane's history view: the band reads it, not the live screen.
  const holder = document.createElement("div");
  holder.style.cssText = "position:absolute;left:-9999px;width:400px;height:600px";
  document.body.appendChild(holder);
  const h = new window.lab.Terminal({ fontFamily: "monospace", fontSize: 14, lineHeight: 1, cols: t.cols, rows: t.rows });
  h.open(holder);
  await new Promise((r) => { let s = ""; for (let i = 1; i <= 120; i++) s += `pane ${i}\r\n`; h.write(s, r); });
  h.scrollLines(-10);
  history = h;
  b.draw();
  await frames(3);
  out.history = look(h);
  history = null;
  // A program on its alternate screen, with no history view: nothing above it.
  await new Promise((r) => t.write("\x1b[?1049h", r));
  await frames(3);
  out.alternate = look(null).n;
  await new Promise((r) => t.write("\x1b[?1049l", r));
  delete box.dataset.folded;
  await frames(2);
  out.unfolded = { n: Number(el.dataset.rows), children: el.children.length };
  b.stop();
  h.dispose();
  holder.remove();
  box.style.height = "600px";
  document.querySelector("#stick").style.paddingTop = "";
  return { ...out, resizes: window.resizes };
});
console.log(`scroller gate: 32 band: ${JSON.stringify(band)}`);
for (const k of ["bottom", "up", "history"]) {
  if (band[k].n !== 2) fail(`32 ${k}: the band drew ${band[k].n} rows, want 2`);
  if (!band[k].same) fail(`32 ${k}: the band's rows are not the two lines above the viewport (${JSON.stringify(band[k].rows)})`);
}
if (!band.bottom.coloured) fail("32: the band's rows lost their colours");
if (band.bottom.rows[0] === band.up.rows[0]) fail("32: the band did not follow a scroll");
if (band.shift !== "translateY(5px)" || band.unshift !== "") fail(`32: the band did not take the grid's part-row shift (${band.shift} / ${band.unshift})`);
if (band.alternate !== 0) fail(`32: a program's alternate screen left ${band.alternate} rows in the band`);
if (band.unfolded.n !== 0 || band.unfolded.children !== 0) fail(`32: unfolding left the band drawn (${JSON.stringify(band.unfolded)})`);
if (band.resizes !== 0) fail(`32: the band resized the terminal ${band.resizes} times`);

// ── 42 ──
const swipe = await page.evaluate(async () => {
  const scroller = document.querySelector("#scroller");
  const field = document.createElement("textarea");
  document.body.appendChild(field);
  let up = true;
  let lowered = 0;
  const stop = window.lab.swipeLowers(scroller, field, { up: () => up, edge: () => 500, lower: () => lowered++ });
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const touch = (type, x, y, target = scroller) => {
    const t = new Touch({ identifier: 1, target, clientX: x, clientY: y });
    const touches = type === "touchend" ? [] : [t];
    target.dispatchEvent(new TouchEvent(type, { touches, changedTouches: [t], bubbles: true, cancelable: true }));
  };
  const drag = async (points, target) => {
    touch("touchstart", points[0][0], points[0][1], target);
    for (const [x, y, pause] of points.slice(1)) {
      if (pause) await wait(pause);
      touch("touchmove", x, y, target);
    }
    touch("touchend", points[points.length - 1][0], points[points.length - 1][1], target);
  };
  const out = {};
  const count = async (name, points, target) => {
    const was = lowered;
    await drag(points, target);
    out[name] = lowered - was;
  };
  // Slow, paused, and short of the edge: nothing.
  await count("short", [[200, 300], [200, 330, 300], [200, 360, 600], [200, 480]]);
  // Slow and paused (300 px in 1.8 s: 0.17 px/ms, under item 28's 0.3
  // average that failed the owner), across the edge: lowered once, and
  // only once.
  await count("slow", [[200, 300], [200, 330, 300], [200, 360, 1500], [200, 505], [200, 560], [200, 600]]);
  // More across than down, past the edge's height: nothing.
  await count("across", [[20, 300], [120, 350], [380, 520]]);
  // Upward: nothing.
  await count("upward", [[200, 580], [200, 400]]);
  // No keyboard up: nothing.
  up = false;
  await count("down", [[200, 300], [200, 520]]);
  up = true;
  // From the bar: nothing.
  const bar = document.createElement("div");
  bar.setAttribute("data-session-bar", "");
  scroller.appendChild(bar);
  await count("bar", [[200, 300], [200, 520]], bar);
  bar.remove();
  stop();
  await count("stopped", [[200, 300], [200, 520]]);
  field.remove();
  return out;
});
console.log(`scroller gate: 42 swipe: ${JSON.stringify(swipe)}`);
if (swipe.slow !== 1) fail(`42: a slow, paused drag across the keyboard's edge lowered it ${swipe.slow} times, want 1`);
for (const k of ["short", "across", "upward", "down", "bar", "stopped"]) {
  if (swipe[k] !== 0) fail(`42 ${k}: lowered the keyboard ${swipe[k]} times, want 0`);
}

await browser.close();
if (bad) process.exit(1);
console.log("scroller gate: a resize keeps the bottom, output never moves the scroller under a finger, a program's wheel follows the travel, the folded deck's band shows the lines above, and the swipe lowers the keyboard at its edge");
