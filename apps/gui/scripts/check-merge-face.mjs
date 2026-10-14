// **The merge's 保留 / 放弃 buttons have a face, not a hole.**
//
// The per-chunk buttons of `@codemirror/merge` are laid absolutely at
// the end of a deleted chunk — over the very lines they are about. The
// package paints its own green and red by `name=accept` / `name=reject`
// (`baseTheme`); khor hands in its own button instead, so every colour
// on it is khor's, and the one that was wrong was the ground:
// `--fill-strong`, which is a **translucent** grey (0.1 light, 0.15
// dark). A wash is right for a track or a picked row and wrong for
// anything printed over text — the code under the buttons showed
// through and mixed with the labels, two glyph sets in one rectangle.
// Reported by the owner with a screenshot, 2026-10-14.
//
// **Why not a contrast gate.** The label never lost its contrast: the
// ink is opaque, and `--foreground` composited over a tenth of the
// ground still reads about 19:1. A contrast gate stays green through
// this whole bug. What failed was that the ground was a hole, so what
// this gate reads is the ground itself.
//
// Claims (each red on the code before the fix):
//   1. every button's own background is **fully opaque** (alpha 1) and
//      is the token it is meant to be for that theme;
//   2. the deleted code really is under the buttons — otherwise 1 is
//      vacuous and this gate could pass on a layout that floats them in
//      blank space;
//   3. **the pixels are the face.** The colour covering most of each
//      button is read back from a screenshot and must be the face. With
//      a translucent ground that colour is the deleted lines' wash
//      (measured: [200, 151, 148] where the face is white), so the
//      claim separates the two states by more than 50 of 255.
//   4. the label's ink still clears the floor on that face.
//
// What it does not judge, on purpose: whether the buttons are the right
// width, or whether a long line running under them reads well. That is
// placement, and placement is on the real-app walk at the end of a
// round (docs/design-system/guidelines/30-behavior.md,「什么配门」).
//
// Run: `npm run merge-face` (starts its own vite, needs system Chrome).
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { writeFileSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const gui = fileURLToPath(new URL("..", import.meta.url));
const { chromium } = createRequire(import.meta.url)("playwright-core");
const port = Number(process.env.MERGE_FACE_PORT ?? 1475);
const entry = `${gui}/scripts/.merge-face-${process.pid}.tsx`;
const pause = (ms) => new Promise((r) => setTimeout(r, ms));

// A conflict whose deleted lines run the width of the column, so the
// buttons really do sit on top of them (`wrap: false` — the shape the
// screenshot was taken in).
const N = 6;
const ROW = "x".repeat(150);
const disk = Array.from({ length: N }, () => ROW).join("\n") + "\n";
const box = Array.from({ length: N }, () => "y".repeat(150)).join("\n") + "\n";

/** `rgb(r, g, b)` / `rgba(r, g, b, a)` / `#rrggbb` as [r, g, b, a]. */
const rgba = (s) => {
  const hex = s.match(/^#([0-9a-f]{6})$/i);
  if (hex) return [0, 2, 4].map((i) => parseInt(hex[1].slice(i, i + 2), 16)).concat(1);
  const m = s.match(/rgba?\(([^)]+)\)/);
  if (!m) return null;
  const p = m[1].split(/[\s,/]+/).filter(Boolean).map(Number);
  return [p[0], p[1], p[2], p.length > 3 ? p[3] : 1];
};
/** WCAG relative luminance of an opaque [r, g, b] in 0..255. */
const luminance = ([r, g, b]) =>
  [r, g, b].reduce((s, v, i) => {
    const c = v / 255;
    return s + [0.2126, 0.7152, 0.0722][i] * (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  }, 0);
const contrast = (a, b) => {
  const [la, lb] = [luminance(a), luminance(b)];
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
};
/** `top` (with alpha) painted over the opaque `below`. */
const over = (top, below) => top.slice(0, 3).map((v, i) => v * top[3] + below[i] * (1 - top[3]));

writeFileSync(
  entry,
  `import React from 'react'; import {createRoot} from 'react-dom/client'; import {flushSync} from 'react-dom';
import {CodeEditor} from '../src/components/CodeEditor.tsx';
import '../src/styles/tokens.css'; import '../src/styles/app.css';
const root=createRoot(document.getElementById('root'));
window.show=(theme)=>flushSync(()=>root.render(
  <div className="flex h-screen flex-col" style={{width:'900px'}}>
    <CodeEditor key={theme} path="/src/thing.ts" text={${JSON.stringify(box)}} editable wrap={false} against={${JSON.stringify(disk)}} onText={()=>{}} onSave={()=>{}} onChunks={()=>{}} />
  </div>));
window.show('dark');`,
);

let vite, browser, bad = 0;
const fail = (m) => { console.error(`merge-face: ${m}`); bad++; };
try {
  vite = spawn("node_modules/.bin/vite", ["--port", String(port), "--strictPort"], { cwd: gui, stdio: ["ignore", "pipe", "pipe"] });
  let ready = false;
  vite.stdout.on("data", (d) => { ready ||= String(d).includes(`:${port}`); });
  for (let i = 0; i < 150 && !ready; i++) await pause(100);
  assert(ready, "vite did not start");

  browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server"] });
  const page = await browser.newPage({ viewport: { width: 1200, height: 700 } });
  const pageErrors = [];
  page.on("pageerror", (e) => pageErrors.push(e.message));
  await page.route(`**/__merge_face.html*`, (route) =>
    route.fulfill({
      contentType: "text/html",
      body: `<!doctype html><html><body><div id="root"></div>
<script type="module">import RefreshRuntime from '/@react-refresh';RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script>
<script type="module" src="/scripts/.merge-face-${process.pid}.tsx"></script></body></html>`,
    }),
  );
  await page.goto(`http://localhost:${port}/__merge_face.html`);
  await page.locator(".cm-chunkButtons .cm-merge-btn").first().waitFor();

  for (const theme of ["light", "dark"]) {
    await page.evaluate((t) => { document.documentElement.setAttribute("data-theme", t); window.show(t); }, theme);
    await pause(300);

    const geo = await page.evaluate(() => {
      const cs = (el) => getComputedStyle(el);
      const root = cs(document.documentElement);
      const chunk = document.querySelector(".cm-deletedChunk");
      // The deleted text's glyph boxes (a Range, not the line's box: a
      // deleted line's div is the column's width either way).
      const glyphs = [];
      for (const line of chunk.querySelectorAll(".cm-deletedLine")) {
        const w = document.createTreeWalker(line, NodeFilter.SHOW_TEXT);
        for (let n = w.nextNode(); n; n = w.nextNode()) {
          const rg = document.createRange();
          rg.selectNodeContents(n);
          for (const r of rg.getClientRects()) if (r.width > 0 && r.height > 0) glyphs.push({ l: r.left, r: r.right, t: r.top, b: r.bottom });
        }
      }
      const buttons = [...document.querySelectorAll(".cm-chunkButtons .cm-merge-btn")].map((b) => {
        const s = cs(b);
        const r = b.getBoundingClientRect();
        return {
          label: b.textContent,
          chunk: b.dataset.mergeChunk,
          bg: s.backgroundColor,
          color: s.color,
          rect: { x: r.left, y: r.top, w: r.width, h: r.height },
          under: glyphs.filter((g) => g.l < r.right && g.r > r.left && g.t < r.bottom && g.b > r.top).length,
        };
      });
      return {
        buttons,
        popover: root.getPropertyValue("--popover").trim(),
        background: root.getPropertyValue("--background").trim(),
        lineWash: cs(chunk.querySelector(".cm-deletedLine")).backgroundColor,
        chunkWash: cs(chunk).backgroundColor,
        deletedTextBg: cs(chunk.querySelector(".cm-deletedText") ?? chunk).backgroundColor,
      };
    });

    const face = rgba(geo.popover);
    for (const b of geo.buttons) {
      const where = `${theme} ${b.label} (${b.chunk})`;
      const bg = rgba(b.bg);
      const ink = rgba(b.color);
      if (bg === null) { fail(`${where}: the background is not a colour (${b.bg})`); continue; }

      // Every claim is read even when an earlier one has failed: on a
      // translucent ground they are all true statements about the same
      // defect, and a gate that stops at the first is a gate that
      // teaches whoever reads it only a third of the reason.
      const bad = (m) => `${where}: ${m}`;
      // 1 — a face, not a wash.
      if (bg[3] !== 1) fail(bad(`the ground is translucent (${b.bg}) — the code under the button reaches the screen through it`));
      if (bg.slice(0, 3).join() !== face.slice(0, 3).join()) fail(bad(`the ground is ${b.bg}, not the floating face --popover (${geo.popover})`));

      // 2 — the text is really under it, or 1 proves nothing.
      if (!(b.under > 0)) fail(bad(`no deleted glyph falls under the button — an opaque ground here would be vacuous`));

      // 4 — the label still reads on that face.
      const ratio = contrast(over(ink, face), face);
      if (ratio < 4.5) fail(bad(`the label measures ${ratio.toFixed(2)}:1 on the face, under 4.5`));

      // 3 — **the pixels.** The same claim read off the screen rather
      // than out of the stylesheet: take the most common colour inside
      // the button and it must *be* the face.
      //
      // Which colour wins is exactly what a translucent ground changes.
      // With a face, the ground is the most common thing on the button
      // and the label's ink is the minority; with `--fill-strong` the
      // deleted lines' wash is composited through the ground, so the
      // colour that covers the button is the wash's, off by more than
      // 30 of 255 — and the label's ink, which is opaque, no longer
      // sits on what the stylesheet says it does.
      //
      // The crop is the button's interior, 6px inside the short edges
      // and 4px inside the long ones, so neither the 4px corner radii
      // nor the neighbouring button is in it.
      const at = geo.buttons.indexOf(b);
      const box = await page.locator(".cm-chunkButtons .cm-merge-btn").nth(at).boundingBox();
      const crop = { x: Math.ceil(box.x) + 6, y: Math.ceil(box.y) + 4, width: Math.floor(box.width) - 12, height: Math.floor(box.height) - 8 };
      const png = (await page.screenshot({ clip: crop })).toString("base64");
      const common = await page.evaluate(async (b64) => {
        const img = await new Promise((r) => { const i = new Image(); i.onload = () => r(i); i.src = "data:image/png;base64," + b64; });
        const c = document.createElement("canvas");
        c.width = img.width; c.height = img.height;
        const ctx = c.getContext("2d");
        ctx.drawImage(img, 0, 0);
        const d = ctx.getImageData(0, 0, c.width, c.height).data;
        const seen = new Map();
        for (let i = 0; i < d.length; i += 4) {
          const k = `${d[i]},${d[i + 1]},${d[i + 2]}`;
          seen.set(k, (seen.get(k) ?? 0) + 1);
        }
        const [colour, count] = [...seen.entries()].sort((a, b) => b[1] - a[1])[0];
        return { colour: colour.split(",").map(Number), count, total: c.width * c.height, distinct: seen.size };
      }, png);
      const [r, g, bl] = common.colour;
      if (Math.abs(r - face[0]) > 3 || Math.abs(g - face[1]) > 3 || Math.abs(bl - face[2]) > 3) {
        fail(
          bad(
            `what covers the button is [${common.colour.join(", ")}] (${common.count} of ${common.total} pixels), ` +
              `not the face --popover [${face.slice(0, 3).join(", ")}] — the ground is not what the stylesheet claims, ` +
              `and the code underneath is composited into it`,
          ),
        );
      }
    }

    console.log(
      `merge-face ${theme}: face --popover ${geo.popover} on ground ${geo.background}; ` +
        `deleted line wash ${geo.lineWash}, chunk wash ${geo.chunkWash}, deletedText ${geo.deletedTextBg}; ${geo.buttons.length} buttons`,
    );
  }

  assert.deepEqual(pageErrors, [], `page errors: ${pageErrors.join(" | ")}`);
} finally {
  if (browser) await browser.close();
  if (vite) vite.kill();
  try { rmSync(entry); } catch {}
}
if (bad) { console.error(`merge-face: ${bad} problem(s)`); process.exit(1); }
console.log("merge-face: ok — the per-chunk buttons are painted on the app's floating face, and nothing shows through");
