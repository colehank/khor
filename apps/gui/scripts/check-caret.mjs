// **The file editor's caret, as VS Code draws it** (the owner 2026-10-16:
// 「光标不够显眼」「按 VS Code 来」), and the commit annotation at the end
// of its line (`lib/code/blame`) never in its way.
//
// The caret has been CodeMirror's since 2026-10-13 (`drawSelection`,
// CodeEditor's head comment). VS Code's default line caret
// (`viewCursor.ts`, `viewCursors.ts`): 2 wide, shown for 500 ms after
// every move and then toggled every 500 ms — so it holds still while
// keys are pressed and blinks once they stop; as tall as the row
// (`editor.cursorHeight` 0, the owner's call 2026-10-16 in place of
// 10-13's ideograph ink, `lib/code/theme`).
//
// Claims (each red on the code before 2026-10-16, numbers below):
//   1. **the painted bar is `--caret-w` wide, where the glyphs end, and
//      inside its own element** — read back from pixels: two
//      screenshots, the caret layer shown and hidden, and the columns
//      that differ are the caret. At the end of lines reached by a
//      click, the arrows, End, typing, Backspace and Enter, with the
//      blame annotation on the caret's line and without it, in WebKit
//      (the app's engine) and Chrome. Before: 1 wide (CodeMirror's
//      1.2 border, `::before` drawn outside the element at `right:
//      100%`).
//   2. **the blink is VS Code's**: one period is 1000 ms (500 shown,
//      500 hidden); a key restarts it in the shown half; keys 200 ms
//      apart for 1.2 s never catch it hidden; 750 ms after the last key
//      it is hidden. Before: 1200 ms.
//   3. **a press on the annotation is a press on the blank after the
//      line**: the caret goes to the line's end and the next key lands
//      there. Before: the caret stayed at column 3 and the key was lost
//      (WebKit put the DOM selection inside the annotation's text,
//      which takes no input, and CodeMirror ignores the widget's events).
//   4. **the annotation stays inside the text column** when the caret
//      moves between two lines of one commit. Before: CodeMirror kept
//      the first line's widget (`eq` by commit alone) with the first
//      line's measured room, and it ran past the column's edge.
//   5. **the painted bar is its line box, top to bottom** — read back
//      from pixels as 1 is, against the caret's row: its line block cut
//      into rows of the line's height, which is `--text-mono--line-
//      height` resolved where the editor stands. Every line end of 1,
//      and an empty line, a line's start, inside and at the start of a
//      CJK line, every row of a wrapped line (Home, End, ↓) and the line
//      a newline after it opens; at ×1, `--ui-scale` 23/17 and the file
//      sizes ×1.15 and ×1.5, with the annotation and without. Before:
//      the ideograph's ink, 12 (WebKit) / 11.5 (Chrome) high on a row
//      of 20, 16 / 15.5 on 27, 13 on 23, 17 / 17.5 on 30.
//
// What it does not judge: whether the caret reads well against a
// particular file, or the long-press card on a phone — the real-app
// walk (docs/LEDGER.md 机). And the owner's 「行末光标偶发消失」 was
// never reproduced here (2026-10-16): claim 1's sweep is where it would
// show, and it never did, before the change or after.
//
// Run: `npm run caret` (starts its own vite; WebKit from Playwright's
// browsers — `PLAYWRIGHT_BROWSERS_PATH`, default the one on disk0 — and
// system Chrome).
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync, rmSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { inflateSync } from "node:zlib";

const DISK0 = "/Volumes/disk0/khor-cm-caret-work/pw-browsers";
if (!process.env.PLAYWRIGHT_BROWSERS_PATH && existsSync(DISK0)) process.env.PLAYWRIGHT_BROWSERS_PATH = DISK0;
// After the variable: playwright reads it when it loads.
const { chromium, webkit } = await import("playwright-core");

const gui = fileURLToPath(new URL("..", import.meta.url));
const port = Number(process.env.CARET_PORT ?? 1481);
const entry = `${gui}/scripts/.caret-gate-${process.pid}.tsx`;
const pause = (ms) => new Promise((r) => setTimeout(r, ms));
const DPR = 2;

const SRC = [
  "// The save road: a write per pause.",
  'import { fetchFileText, writeFile } from "@/api";',
  "",
  "export type Draft = { text: string; was: string };",
  "/** 写回缓冲区，钉在读到的那个摘要上。 */",
  "export async function save(path: string, d: Draft) {",
  "  const now = await fetchFileText(path);",
  "  if (now.digest !== d.was) {",
  "    // 别人先写了：停下，让人来选。",
  '    throw new Error("clash");',
  "  }",
  "  return now;   ",
  "}",
  "中文一行，结尾是汉字",
  "x",
  "",
].join("\n");

/** Just enough PNG to read Playwright's screenshots back (8-bit RGB/RGBA). */
function png(buf) {
  let p = 8, w = 0, h = 0, ct = 0;
  const idat = [];
  while (p < buf.length) {
    const len = buf.readUInt32BE(p), type = buf.toString("ascii", p + 4, p + 8), d = buf.subarray(p + 8, p + 8 + len);
    if (type === "IHDR") [w, h, ct] = [d.readUInt32BE(0), d.readUInt32BE(4), d[9]];
    if (type === "IDAT") idat.push(d);
    p += 12 + len;
  }
  const bpp = ct === 6 ? 4 : 3, raw = inflateSync(Buffer.concat(idat)), stride = w * bpp, px = Buffer.alloc(h * stride);
  for (let y = 0; y < h; y++) {
    const f = raw[y * (stride + 1)], line = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    for (let x = 0; x < stride; x++) {
      const a = x >= bpp ? px[y * stride + x - bpp] : 0, b = y ? px[(y - 1) * stride + x] : 0, c = y && x >= bpp ? px[(y - 1) * stride + x - bpp] : 0;
      let v = line[x];
      if (f === 1) v += a;
      else if (f === 2) v += b;
      else if (f === 3) v += (a + b) >> 1;
      else if (f === 4) {
        const q = a + b - c, pa = Math.abs(q - a), pb = Math.abs(q - b), pc = Math.abs(q - c);
        v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      }
      px[y * stride + x] = v & 255;
    }
  }
  return { w, h, bpp, px };
}
/** The device columns and rows where two shots differ (by more than
    antialiasing). */
function differing(a, b) {
  const cols = new Set(), rows = new Set();
  for (let y = 0; y < a.h; y++)
    for (let x = 0; x < a.w; x++) {
      const i = (y * a.w + x) * a.bpp;
      if (Math.abs(a.px[i] - b.px[i]) + Math.abs(a.px[i + 1] - b.px[i + 1]) + Math.abs(a.px[i + 2] - b.px[i + 2]) > 90) {
        cols.add(x);
        rows.add(y);
      }
    }
  const up = (s) => [...s].sort((p, q) => p - q);
  return { cols: up(cols), rows: up(rows) };
}

const bad = [];
const claim = (ok, what) => {
  if (!ok) bad.push(what);
};

let vite, browser;
try {
  writeFileSync(
    entry,
    `import React from 'react'; import {createRoot} from 'react-dom/client';
import {CodeEditor} from '../src/components/CodeEditor.tsx';
import {getDrawSelectionConfig} from '@codemirror/view';
import '../src/styles/tokens.css'; import '../src/styles/app.css';
window.__drawConfig=getDrawSelectionConfig;
const C1={id:'7a921aa4c0ffee',author:'colehank',when_ms:Date.now()-3*86400000,subject:'save road'};
const C2={id:'3f00d1e2b7',author:'net-d',when_ms:Date.now()-5*3600000,subject:'retry'};
const q=new URLSearchParams(location.search), blame=!q.has('noblame');
function App(){const [t,setT]=React.useState(window.__text);
 const blamed=React.useMemo(()=>blame?t.split('\\n').map(l=>l.startsWith('//')?C2:C1):null,[t]);
 return <CodeEditor path='lib/save.ts' text={t} editable blamed={blamed} onText={setT}/>;}
createRoot(document.getElementById('root')).render(<App/>);`,
  );
  vite = spawn("node_modules/.bin/vite", ["--port", String(port), "--strictPort"], { cwd: gui, stdio: ["ignore", "pipe", "pipe"], env: { ...process.env, KHOR_FACE_GATE: "1" } });
  let ready = false;
  vite.stdout.on("data", (d) => {
    ready ||= String(d).includes(`:${port}`);
  });
  for (let i = 0; i < 100 && !ready; i++) await pause(100);
  assert(ready, "vite did not start");

  for (const engine of ["webkit", "chrome"]) {
    try {
      browser = engine === "webkit" ? await webkit.launch({ headless: true }) : await chromium.launch({ channel: "chrome", headless: true });
    } catch (e) {
      bad.push(`${engine}: cannot launch (${String(e.message).split("\n")[0]}) — not a pass`);
      continue;
    }
    for (const mode of ["blame", "noblame"]) {
      const ctx = await browser.newContext({ viewport: { width: 760, height: 560 }, deviceScaleFactor: DPR });
      const page = await ctx.newPage();
      const errors = [];
      page.on("pageerror", (e) => errors.push(e.message));
      const text = mode === "blame" ? SRC + Array.from({ length: 6 }, (_, i) => "b".repeat(56 + 3 * i)).join("\n") + "\n" : SRC;
      await page.addInitScript((t) => (window.__text = t), text);
      await page.route("**/__caret_gate?**", (r) =>
        r.fulfill({
          contentType: "text/html",
          body: `<!doctype html><html data-theme="light"><body><div id="root" class="flex h-screen flex-col"></div><script type="module">import RefreshRuntime from '/@react-refresh';RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/.caret-gate-${process.pid}.tsx"></script></body></html>`,
        }),
      );
      await page.goto(`http://localhost:${port}/__caret_gate?${mode}`);
      await page.locator(".cm-content").waitFor();
      await pause(400);
      const at = `${engine} ${mode}`;

      // ---- 1. the painted bar, swept over line ends ----
      const read = () =>
        page.evaluate(() => {
          const v = document.querySelector("[data-code-view]").__view;
          const { head } = v.state.selection.main, line = v.state.doc.lineAt(head);
          const layer = v.scrollDOM.querySelector(".cm-cursorLayer");
          const cs = layer.querySelectorAll(".cm-cursor"), c = cs[0];
          const r = c?.getBoundingClientRect();
          const g = head > line.from ? v.coordsAtPos(head, -1) : v.coordsAtPos(head, 1);
          const before = c && getComputedStyle(c, "::before");
          // The caret's line box: its block cut into rows of the line's
          // height, and the row the drawn element's middle falls in — a
          // wrapped line is one block of several rows.
          const block = v.lineBlockAt(head), top = v.documentTop + block.top;
          const lh = parseFloat(getComputedStyle(v.contentDOM.querySelector(".cm-line")).lineHeight);
          const k = r ? Math.floor((r.top + r.height / 2 - top) / lh) : 0;
          // The token itself, resolved where the editor stands.
          const probe = document.createElement("div");
          probe.style.height = "var(--text-mono--line-height)";
          v.dom.parentElement.appendChild(probe);
          const token = probe.getBoundingClientRect().height;
          probe.remove();
          return {
            line: line.number, col: head - line.from, len: line.length, n: cs.length,
            box: r && { l: r.left, r: r.right, t: r.top, h: r.height },
            row: { t: top + k * lh, b: top + (k + 1) * lh, k, rows: block.height / lh, lh, token },
            end: g && (head > line.from ? g.right : g.left),
            beforeW: before && parseFloat(before.width),
            caretW: parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--caret-w")),
          };
        });
      const shot = async (clip) => {
        await page.evaluate(() => {
          for (const a of document.querySelector(".cm-cursorLayer").getAnimations()) {
            a.pause();
            a.currentTime = 0;
          }
        });
        const a = png(await page.screenshot({ clip }));
        await page.evaluate(() => (document.querySelector(".cm-cursorLayer").style.visibility = "hidden"));
        const b = png(await page.screenshot({ clip }));
        await page.evaluate(() => {
          const l = document.querySelector(".cm-cursorLayer");
          l.style.visibility = "";
          for (const x of l.getAnimations()) x.play();
        });
        return differing(a, b);
      };
      const heights = [];
      const widths = [];
      const step = async (label) => {
        await pause(80);
        const s = await read();
        if (s.n !== 1 || !s.box || s.end == null) {
          claim(false, `${at} · 1 · ${label}: line ${s.line}:${s.col} — ${s.n} carets drawn`);
          return;
        }
        const clip = { x: s.end - 4, y: s.box.t, width: 8, height: Math.max(1, s.box.h) };
        const { cols } = await shot(clip);
        const w = cols.length / DPR;
        const mid = cols.length ? clip.x + (cols[0] + cols[cols.length - 1] + 1) / 2 / DPR : NaN;
        widths.push(w);
        claim(Math.abs(w - s.caretW) <= 0.5, `${at} · 1 · ${label}: line ${s.line}:${s.col}/${s.len} painted ${w} wide, --caret-w is ${s.caretW}`);
        claim(Math.abs(mid - s.end) <= 1, `${at} · 1 · ${label}: line ${s.line}:${s.col}/${s.len} painted at ${mid?.toFixed(2)}, the glyphs end at ${s.end.toFixed(2)}`);
        claim(s.beforeW === s.caretW && s.box.l <= s.end && s.box.r >= s.end, `${at} · 1 · ${label}: the bar (${s.beforeW} wide) is not inside its element [${s.box.l.toFixed(2)}, ${s.box.r.toFixed(2)}] at ${s.end.toFixed(2)}`);
        await tall(label, s);
      };
      // ---- 5. the bar as tall as its line box ----
      const tall = async (label, s) => {
        if (!s) {
          await pause(80);
          s = await read();
        }
        if (s.n !== 1 || !s.box) {
          claim(false, `${at} · 5 · ${label}: line ${s.line}:${s.col} — ${s.n} carets drawn`);
          return;
        }
        const { row } = s;
        // Whole pixels: WebKit's screenshot rounds a clip's origin.
        const y = Math.max(0, Math.floor(row.t - row.lh / 2));
        const clip = { x: s.box.l - 2, y, width: s.box.r - s.box.l + 4, height: Math.min(page.viewportSize().height, row.b + row.lh / 2) - y };
        const { rows } = await shot(clip);
        const t = rows.length ? clip.y + rows[0] / DPR : NaN, b = rows.length ? clip.y + (rows[rows.length - 1] + 1) / DPR : NaN;
        heights.push(b - t);
        const where = `${at} · 5 · ${label}: line ${s.line}:${s.col}/${s.len}${row.rows > 1 ? ` (row ${row.k + 1} of ${row.rows})` : ""}`;
        claim(Math.abs(row.lh - row.token) <= 0.5 && Math.abs(row.rows - Math.round(row.rows)) < 0.01, `${where}: rows ${row.lh} high over ${row.rows}, --text-mono--line-height is ${row.token}`);
        claim(Math.abs(t - row.t) <= 0.5 && Math.abs(b - row.b) <= 0.5, `${where}: painted ${t.toFixed(2)}–${b.toFixed(2)} (${(b - t).toFixed(2)} high), the line box is ${row.t.toFixed(2)}–${row.b.toFixed(2)} (${row.token})`);
      };
      const endOf = (n) =>
        page.evaluate((n) => {
          const v = document.querySelector("[data-code-view]").__view;
          const l = v.state.doc.line(n), c = v.coordsAtPos(l.to, -1) ?? v.coordsAtPos(l.to, 1);
          return { x: Math.min(c.right + 8, v.contentDOM.getBoundingClientRect().right - 8), y: (c.top + c.bottom) / 2 };
        }, n);
      for (const n of [2, 5, 9, 12, 14, 15, 3]) {
        const p = await endOf(n);
        await page.mouse.click(p.x, p.y);
        await step(`click after line ${n}`);
      }
      await page.mouse.click(...Object.values(await endOf(1)));
      for (let i = 0; i < 4; i++) {
        await page.keyboard.press("ArrowDown");
        await page.keyboard.press("End");
        await step("↓ End");
      }
      for (const ch of "ab") {
        await page.keyboard.type(ch);
        await step(`typed ${ch}`);
      }
      await page.keyboard.press("Backspace");
      await step("⌫");
      await page.keyboard.press("Enter");
      await step("⏎");
      await page.keyboard.press("Backspace");
      await step("⌫ back up");
      await page.keyboard.press("ArrowUp");
      await page.keyboard.press("End");
      await step("↑ End");

      // ---- 2. the blink ----
      if (mode === "blame") {
        const blink = () =>
          page.evaluate(() => {
            const v = document.querySelector("[data-code-view]").__view;
            const l = v.scrollDOM.querySelector(".cm-cursorLayer"), a = l.getAnimations()[0];
            return { rate: window.__drawConfig(v.state).cursorBlinkRate, period: a ? a.effect.getComputedTiming().duration : null, t: a ? a.currentTime : null, opacity: getComputedStyle(l).opacity };
          });
        await page.keyboard.press("ArrowLeft");
        const first = await blink();
        claim(first.period === 1000, `${at} · 2: one blink is ${first.period} ms (VS Code: 500 shown + 500 hidden)`);
        claim(first.t !== null && first.t < 250, `${at} · 2: a key did not restart the blink (${first.t} ms in)`);
        const typing = [];
        for (let i = 0; i < 6; i++) {
          await page.keyboard.press(i % 2 ? "ArrowLeft" : "ArrowRight");
          await pause(200);
          typing.push((await blink()).opacity);
        }
        claim(typing.every((o) => o === "1"), `${at} · 2: hidden while keys came 200 ms apart: ${typing.join(" ")}`);
        await pause(550);
        const idle = await blink();
        claim(idle.opacity === "0", `${at} · 2: still shown 750 ms after the last key (opacity ${idle.opacity}, ${idle.t} ms in)`);
      }

      // ---- 3. a press on the annotation ----
      if (mode === "blame") {
        await page.evaluate(() => {
          const v = document.querySelector("[data-code-view]").__view;
          v.focus();
          v.dispatch({ selection: { anchor: v.state.doc.line(7).from + 3 }, userEvent: "select" });
        });
        await pause(80);
        const lab = await page.evaluate(() => {
          const r = document.querySelector("[data-blame-inline]").getBoundingClientRect();
          return { x: r.left + Math.min(r.width - 4, 60), y: r.top + r.height / 2 };
        });
        await page.mouse.click(lab.x, lab.y);
        await pause(80);
        const s = await read();
        claim(s.line === 7 && s.col === s.len, `${at} · 3: a press on line 7's annotation left the caret at ${s.line}:${s.col}, not the line's end (${s.len})`);
        await page.keyboard.type("Z");
        await pause(80);
        const t = await page.evaluate(() => document.querySelector("[data-code-view]").__view.state.doc.line(7).text);
        claim(t.endsWith("Z"), `${at} · 3: the key after the press did not land at the line's end: ${JSON.stringify(t)}`);
        await page.keyboard.press("Backspace");
      }

      // ---- 4. the annotation inside the column ----
      if (mode === "blame") {
        const fits = await page.evaluate(async () => {
          const v = document.querySelector("[data-code-view]").__view;
          const frame = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
          const edge = () => {
            const c = v.contentDOM.getBoundingClientRect().right - (parseFloat(getComputedStyle(v.contentDOM).paddingRight) || 0);
            const line = v.contentDOM.querySelector(".cm-blame")?.closest(".cm-line");
            return c - (line ? parseFloat(getComputedStyle(line).paddingRight) || 0 : 0);
          };
          const go = async (n) => {
            v.dispatch({ selection: { anchor: v.state.doc.line(n).to }, userEvent: "select" });
            await frame();
            const lab = v.contentDOM.querySelector("[data-blame-inline]");
            return { n, right: lab.getBoundingClientRect().right, edge: edge(), text: lab.textContent };
          };
          const out = [];
          // From a short line of the commit to each long one, and back.
          for (let n = v.state.doc.lines - 6; n < v.state.doc.lines; n++) {
            await go(15);
            out.push(await go(n));
          }
          return out;
        });
        for (const f of fits) claim(f.right <= f.edge + 0.5, `${at} · 4: line ${f.n}'s annotation 「${f.text}」 ends at ${f.right.toFixed(1)}, past the column's edge ${f.edge.toFixed(1)}`);
      }
      // ---- 5. the rest of the places a caret stands, then larger type ----
      // Last: it adds a wrapped line to the end, which 4 counts back from.
      const put = (n, col) =>
        page.evaluate(([n, col]) => {
          const v = document.querySelector("[data-code-view]").__view;
          v.focus();
          v.dispatch({ selection: { anchor: v.state.doc.line(n).from + col }, userEvent: "select", scrollIntoView: true });
        }, [n, col]);
      const sizes = [
        ["×1", null],
        ["--ui-scale 23/17", () => document.documentElement.style.setProperty("--ui-scale", String(23 / 17))],
        ["file ×1.15", () => {
          document.documentElement.style.removeProperty("--ui-scale");
          const r = document.getElementById("root").style;
          r.setProperty("--text-mono", "calc(var(--fs-mono) * 1.15)");
          r.setProperty("--text-mono--line-height", "round(calc(var(--lh-mono) * 1.15), var(--pixel))");
        }],
        ["file ×1.5", () => {
          const r = document.getElementById("root").style;
          r.setProperty("--text-mono", "calc(var(--fs-mono) * 1.5)");
          r.setProperty("--text-mono--line-height", "round(calc(var(--lh-mono) * 1.5), var(--pixel))");
        }],
      ];
      const WRAP = "const road = save(path, draft); // 写回缓冲区，钉在读到的那个摘要上。".repeat(4);
      for (const [size, set] of sizes) {
        if (set) {
          await page.evaluate(set);
          await page.evaluate(() => document.querySelector("[data-code-view]").__view.requestMeasure());
          await pause(300);
        }
        await put(3, 0);
        await tall(`${size} · empty line`);
        await put(6, 0);
        await tall(`${size} · line start`);
        await put(14, 3);
        await tall(`${size} · inside CJK`);
        await put(14, 0);
        await tall(`${size} · CJK line start`);
        await put(2, 0);
        await pause(80);
        const p = await endOf(2);
        await page.mouse.click(p.x, p.y);
        await step(`${size} · click after line 2`);
        if (!set) {
          await page.evaluate((w) => {
            const v = document.querySelector("[data-code-view]").__view;
            v.dispatch({ changes: { from: v.state.doc.length, insert: `\n${w}` } });
          }, WRAP);
          await pause(80);
        }
        const last = await page.evaluate(() => document.querySelector("[data-code-view]").__view.state.doc.lines);
        await put(last, 0);
        await pause(80);
        const w = await read();
        claim(w.row.rows >= 3, `${at} · 5 · ${size}: the long line did not wrap (${w.row.rows} rows)`);
        await tall(`${size} · wrapped, row 1 start`, w);
        for (let i = 0; i < 2; i++) {
          await page.keyboard.press("End");
          await tall(`${size} · wrapped, End`);
          await page.keyboard.press("ArrowDown");
          await tall(`${size} · wrapped, ↓`);
        }
        await page.keyboard.press("Home");
        await tall(`${size} · wrapped, Home`);
      }
      await page.keyboard.press("Enter");
      await tall("⏎ after the wrapped line");
      claim(errors.length === 0, `${at}: page errors ${JSON.stringify(errors)}`);
      console.log(`caret gate · ${at}: bar ${[...new Set(widths)].join("/")} wide over ${widths.length} line ends; ${[...new Set(heights.map((h) => h.toFixed(1)))].join("/")} high over ${heights.length} places`);
      await ctx.close();
    }
    await browser.close();
    browser = null;
  }
} finally {
  await browser?.close();
  vite?.kill();
  rmSync(entry, { force: true });
}
if (bad.length) {
  for (const b of bad) console.error(`caret gate: ${b}`);
  console.error(`caret gate: ${bad.length} red`);
  process.exit(1);
}
console.log("caret gate: clean");
