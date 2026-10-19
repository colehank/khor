// The notebook gate (lane nb-view): what the `.ipynb` preview
// (`components/Notebook`) must and must not do, measured in a real
// engine on the lab page (`lab/nb.html`) with nothing else of the app
// around it.
//
// Claims — each a number or a string a machine reads, none a judgment of
// looks:
//   0. probe: the lab booted and drew every cell of the sample;
//   1. MIME: each output is drawn as the one representation nbconvert's
//      order picks (`lab/nb-sample`'s `EXPECTED`): HTML over text, a
//      picture over its text, a script-only bundle's picture or text,
//      stdout and stderr and a traceback as themselves;
//   2. a `text/html` output runs nothing and fetches nothing: no
//      `script` / `iframe` / `style` / `form` / `object` element, no
//      `on*` attribute, no `javascript:` address, no picture with a
//      network address, the `<style>`'s rules not left as words, no
//      request to the network at all, and nothing the attacks would set
//      was set — while its table and its inlined `data:` picture are
//      there;
//   3. an SVG output is an `<img>` with a `data:` address, never an
//      `<svg>` in the page, and its script did not run;
//   4. a script-only output: with a picture, the picture and no
//      sentence; without, its text and the sentence; with neither, the
//      sentence alone — three sentences in the sample;
//   5. ANSI: the traceback's runs are spans in the terminal's colours
//      for the ground — latte on light, mocha on dark, light again under
//      a light preview ground in a dark app — a 256-colour index from
//      xterm's table, an RGB as written, bold as weight; `\r` keeps the
//      last state of a line;
//   6. a markdown cell's `attachment:` picture is an `<img>` with its
//      `data:` address; its formula is KaTeX; its `<script>` is not an
//      element;
//   7. a code cell is coloured (Shiki spans) and wears its count;
//   8. nbformat 3 and a broken file each say one sentence;
//   9. 390 wide: the page does not scroll sideways — the wide line and
//      the wide table scroll inside their own boxes;
//  10. a 300-cell notebook draws only what is near the screen on open,
//      and the last cell once scrolled to (the timing is printed, not
//      judged);
//  11. zero page errors.
//
// Run: `npm run nb` (boots its own vite; Playwright's WebKit when
// `PLAYWRIGHT_BROWSERS_PATH` points at one, else system Chrome; no cargo
// lock).
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

import { chromium, webkit } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const PORT = Number(process.env.NB_PORT ?? 1486);
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

let failures = 0;
const fail = (msg) => {
  failures += 1;
  console.error(`✗ ${msg}`);
};
const ok = (msg) => console.log(`✓ ${msg}`);
const same = (got, want) => JSON.stringify(got) === JSON.stringify(want);

/** Catppuccin, as `term-palettes.json` holds it: the red and bold red the
    traceback asks for, on each ground. */
const PALETTES = JSON.parse(
  await import("node:fs").then((fs) => fs.readFileSync(new URL("../src/lib/term-palettes.json", import.meta.url), "utf8")),
);
const hexToRgb = (h) => {
  const n = parseInt(h.slice(1), 16);
  return `rgb(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255})`;
};

let vite = null;
let browser = null;
try {
  vite = spawn("node_modules/.bin/vite", ["--port", String(PORT), "--strictPort", "--host", "127.0.0.1"], {
    cwd: gui,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let gone = null;
  vite.on("exit", (code) => (gone = `vite exited with ${code} — is ${PORT} already taken?`));
  let ready = false;
  vite.stdout.on("data", (d) => (ready ||= String(d).includes(`:${PORT}`)));
  for (let i = 0; i < 150 && !ready; i++) {
    if (gone) throw new Error(gone);
    await wait(100);
  }
  if (!ready) throw new Error(`vite never came up on ${PORT}`);

  let engine = "webkit";
  try {
    browser = await webkit.launch({ headless: true });
  } catch {
    engine = "chrome";
    browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server"] });
  }
  console.log(`engine: ${engine}`);

  const open = async (query, { width = 1000, height = 800 } = {}) => {
    const page = await browser.newPage({ viewport: { width, height } });
    const errors = [];
    const network = [];
    page.on("pageerror", (e) => errors.push(String(e)));
    page.on("request", (r) => {
      const u = new URL(r.url());
      if (u.protocol !== "data:" && u.hostname !== "127.0.0.1") network.push(r.url());
    });
    // Nothing leaves: a request that slips through is recorded above and
    // answered here, never by the network.
    await page.route(/^https?:\/\/(?!127\.0\.0\.1)/, (route) => route.fulfill({ status: 204, body: "" }));
    // A picture beside the notebook (the HTML output's `src="x"`): not there.
    await page.route("**/api/preview", (route) => route.fulfill({ status: 500, body: "not there" }));
    await page.goto(`http://127.0.0.1:${PORT}/lab/nb.html${query}`);
    await page.waitForFunction(() => typeof window.nbShow === "function", null, { timeout: 60_000 });
    return { page, errors, network };
  };

  /** Draws the sample and waits for its lazy parts (KaTeX, the clean,
      Shiki's grammar) to land. */
  const showSample = async (page) => {
    const text = await page.evaluate(() => window.nbSample());
    await page.evaluate((t) => window.nbShow(t), text);
    await page.waitForFunction(
      () =>
        document.querySelectorAll("[data-nb-out='html'] table").length > 0 &&
        document.querySelectorAll("[data-nb-input] span[style*='--shiki']").length > 0 &&
        document.querySelectorAll("[data-nb] .katex").length > 0,
      null,
      { timeout: 20_000 },
    );
    await wait(300);
  };

  // The first pass through each lazy chunk can make vite re-optimise and
  // reload; touch the sample once so the measured passes run settled.
  {
    const { page } = await open("?light");
    try {
      await showSample(page);
    } catch {
      // A reload mid-way; the next page finds the chunks ready.
    }
    await wait(1500);
    await page.close();
  }

  const { page, errors, network } = await open("?light");
  await showSample(page);
  const expected = await page.evaluate(() => window.nbExpected());

  // 0. probe
  {
    const cells = await page.locator("[data-nb-cell]").count();
    const drawn = await page.locator("[data-nb-drawn]").count();
    if (cells !== expected.length || drawn !== cells) fail(`probe: ${cells} cells (${drawn} drawn), wanted ${expected.length}`);
    else ok(`probe: ${cells} cells drawn`);
  }

  // 1. MIME choice
  {
    const got = await page.$$eval("[data-nb-cell]", (cells) =>
      cells.map((c) => [...c.querySelectorAll("[data-nb-out]")].map((o) => o.getAttribute("data-nb-out"))),
    );
    const bad = got.map((g, i) => (same(g, expected[i]) ? null : `cell ${i}: ${JSON.stringify(g)} ≠ ${JSON.stringify(expected[i])}`)).filter(Boolean);
    if (bad.length) bad.forEach((b) => fail(`mime: ${b}`));
    else ok(`mime: ${got.flat().length} outputs, each drawn as the one nbconvert's order picks`);
  }

  // 2. text/html runs nothing and fetches nothing
  {
    const html = await page.evaluate(() => {
      const root = document.querySelector("[data-nb-out='html']");
      const els = [...root.querySelectorAll("*")];
      return {
        tags: root.querySelectorAll("script, iframe, style, form, object, embed, link, meta, base").length,
        on: els.filter((e) => [...e.attributes].some((a) => /^on/i.test(a.name))).map((e) => e.tagName),
        js: els.filter((e) => [...e.attributes].some((a) => /^\s*javascript:/i.test(a.value))).map((e) => e.tagName),
        remote: [...root.querySelectorAll("img")].filter((i) => /^(https?:)?\/\//i.test(i.getAttribute("src") ?? "")).length,
        table: root.querySelectorAll("table th, table td").length,
        inlined: [...root.querySelectorAll("img")].filter((i) => i.getAttribute("src")?.startsWith("data:image/png")).length,
        styleWords: root.textContent.includes("vertical-align"),
        pwned: window.__nbPwned ?? null,
      };
    });
    const was = failures;
    if (html.tags) fail(`html: ${html.tags} script/iframe/style/form-like element(s)`);
    if (html.on.length) fail(`html: on* attributes on ${html.on.join(", ")}`);
    if (html.js.length) fail(`html: javascript: addresses on ${html.js.join(", ")}`);
    if (html.remote) fail(`html: ${html.remote} picture(s) with a network address`);
    if (html.styleWords) fail("html: the <style>'s rules were left on the page as words");
    if (html.table !== 12) fail(`html: the table has ${html.table} cells, wanted 12`);
    if (html.inlined !== 1) fail(`html: ${html.inlined} inlined data: picture(s), wanted 1`);
    if (html.pwned !== null) fail(`html: something ran — window.__nbPwned = ${html.pwned}`);
    if (network.length) fail(`html: the page asked the network for ${network.join(", ")}`);
    if (failures === was) ok("html: no script, frame, style, form, on*, javascript: or network picture; nothing ran or went out; table and inlined picture kept");
  }

  // 3. SVG is a picture, not markup
  {
    const svg = await page.evaluate(() => ({
      img: [...document.querySelectorAll("[data-nb-out='image'][data-mime='image/svg+xml'] img")].filter((i) => i.src.startsWith("data:image/svg+xml")).length,
      inline: [...document.querySelectorAll("[data-nb] svg")].filter((s) => !s.closest(".katex")).length,
    }));
    if (svg.img !== 1 || svg.inline !== 0) fail(`svg: ${svg.img} data: img, ${svg.inline} inline <svg>`);
    else ok("svg: drawn through <img> from data:, no <svg> in the page");
  }

  // 4. script-only outputs
  {
    const s = await page.$$eval("[data-nb-cell]", (cells) =>
      cells.map((c) => ({
        picture: c.querySelectorAll("[data-nb-out='image']").length,
        text: c.querySelector("[data-nb-out='text']")?.textContent ?? null,
        said: c.querySelectorAll("[data-nb-script]").length,
      })),
    );
    const want = [
      [4, { picture: 1, text: null, said: 0 }],
      [5, { picture: 0, text: "Figure({\n    'data': [], 'layout': {}\n})", said: 1 }],
      [6, { picture: 0, text: "IntSlider(value=3)", said: 1 }],
      [7, { picture: 0, text: null, said: 1 }],
    ];
    const bad = want.filter(([i, w]) => !same(s[i], w));
    const total = s.reduce((n, c) => n + c.said, 0);
    for (const [i, w] of bad) fail(`script: cell ${i} ${JSON.stringify(s[i])} ≠ ${JSON.stringify(w)}`);
    if (total !== 3) fail(`script: ${total} sentences, wanted 3`);
    if (!bad.length && total === 3) ok("script: plotly with a png shows it, plotly / widget show their text and one sentence, bokeh the sentence alone");
  }

  // 5. ANSI
  const ansi = async (p) =>
    p.evaluate(() => {
      const err = document.querySelector("[data-nb-out='error']");
      const span = (text, nth = 0) => [...err.querySelectorAll("[data-ansi-fg]")].filter((s) => s.textContent === text)[nth];
      const style = (s) => (s ? { color: getComputedStyle(s).color, weight: getComputedStyle(s).fontWeight } : null);
      return {
        red: style(span("ZeroDivisionError")),
        boldRed: style(span("ZeroDivisionError", 1)),
        indexed: style(span("1")),
        rgb: style(span("0")),
        plain: err.textContent.includes("\x1b") || err.textContent.includes("[0;31m"),
        progress: document.querySelectorAll("[data-nb-out='stdout']")[1]?.textContent ?? null,
      };
    });
  const ansiOk = (a, pal, where) => {
    const was = failures;
    const red = hexToRgb(PALETTES[pal].theme.red);
    if (a.red?.color !== red) fail(`ansi ${where}: red is ${a.red?.color}, wanted ${pal}'s ${red}`);
    if (a.boldRed?.color !== red || Number(a.boldRed?.weight) < 600) fail(`ansi ${where}: bold red is ${JSON.stringify(a.boldRed)}`);
    if (a.indexed?.color !== "rgb(255, 135, 0)") fail(`ansi ${where}: 38;5;208 is ${a.indexed?.color}, wanted rgb(255, 135, 0)`);
    if (a.rgb?.color !== "rgb(10, 120, 200)") fail(`ansi ${where}: 38;2;10;120;200 is ${a.rgb?.color}`);
    if (a.plain) fail(`ansi ${where}: escape codes left in the text`);
    if (a.progress !== "100%\n") fail(`ansi ${where}: \\r progress shows ${JSON.stringify(a.progress)}, wanted "100%\\n"`);
    if (failures === was) ok(`ansi ${where}: ${pal}'s red, bold, xterm 208, RGB as written, \\r folded`);
  };
  ansiOk(await ansi(page), "latte", "light");

  // 6. markdown cell: attachment, formula, no script
  {
    const m = await page.evaluate(() => {
      const cell = document.querySelector("[data-nb-cell='markdown']");
      return {
        attached: [...cell.querySelectorAll("img")].filter((i) => i.getAttribute("src")?.startsWith("data:image/png")).length,
        katex: cell.querySelectorAll(".katex").length,
        script: cell.querySelectorAll("script").length,
      };
    });
    if (m.attached !== 1 || m.katex < 2 || m.script) fail(`markdown: ${JSON.stringify(m)}`);
    else ok("markdown cell: attachment drawn from data:, formulas by KaTeX, its <script> no element");
  }

  // 7. code cells: coloured and counted
  {
    const c = await page.$$eval("[data-nb-cell='code']", (cells) =>
      cells.map((x) => ({ prompt: x.querySelector("[data-nb-prompt]")?.textContent?.trim(), painted: x.querySelectorAll("[data-nb-input] span[style*='--shiki']").length > 0 })),
    );
    const unpainted = c.filter((x) => !x.painted).length;
    if (c[0]?.prompt !== "[1]" || unpainted) fail(`code: first prompt ${c[0]?.prompt}, ${unpainted} cell(s) not coloured`);
    else ok(`code: ${c.length} cells coloured by Shiki, counts worn ([1] first)`);
  }

  // 8. what cannot be read says so
  {
    const said = async (text) => {
      await page.evaluate((t) => window.nbShow(t), text);
      return page.evaluate(() => {
        const e = document.querySelector("[data-nb-unread]");
        return e ? [e.getAttribute("data-nb-unread"), e.textContent] : null;
      });
    };
    const v3 = await said(await page.evaluate(() => window.nbV3));
    const broken = await said('{"nbformat": 4, "cells": [');
    if (v3?.[0] !== "version" || !v3[1].includes("3") || broken?.[0] !== "json" || !broken[1]) fail(`unread: v3 ${JSON.stringify(v3)}, broken ${JSON.stringify(broken)}`);
    else ok(`unread: 「${v3[1]}」, 「${broken[1]}」`);
  }
  await page.close();

  // 5 again: on dark, and a light preview ground in a dark app.
  for (const [query, pal, where] of [
    ["?dark", "mocha", "dark"],
    ["?dark&ground=light", "latte", "dark app, light ground"],
    ["?light&ground=dark", "mocha", "light app, dark ground"],
  ]) {
    const o = await open(query);
    await showSample(o.page);
    ansiOk(await ansi(o.page), pal, where);
    errors.push(...o.errors);
    await o.page.close();
  }

  // 9. a phone's width
  {
    const o = await open("?light", { width: 390, height: 844 });
    await showSample(o.page);
    const w = await o.page.evaluate(() => {
      const s = document.querySelector("[data-file-preview]");
      const nb = document.querySelector("[data-nb]");
      const wideOut = [...document.querySelectorAll("[data-nb-out='stdout']")].pop();
      return {
        page: document.documentElement.scrollWidth,
        scroller: [s.scrollWidth, s.clientWidth],
        nb: [nb.scrollWidth, nb.clientWidth],
        wide: [wideOut.scrollWidth, wideOut.clientWidth],
      };
    });
    if (w.page > 390 || w.scroller[0] > w.scroller[1] || w.nb[0] > w.nb[1] || !(w.wide[0] > w.wide[1])) fail(`phone: ${JSON.stringify(w)}`);
    else ok(`phone 390: nothing pushes the page sideways; the wide output scrolls in itself (${w.wide[0]} in ${w.wide[1]})`);
    errors.push(...o.errors);
    await o.page.close();
  }

  // 10. a big notebook draws near the screen
  {
    const o = await open("?light", { width: 1000, height: 800 });
    const text = await o.page.evaluate(() => window.nbBig(300));
    const t = await o.page.evaluate((x) => window.nbOpen(x), text);
    const bytes = text.length;
    if (t.cells !== 300 || t.drawn >= 60) fail(`big: ${t.drawn} of ${t.cells} cells drawn on open`);
    else ok(`big: ${t.drawn} of ${t.cells} cells drawn on open (${(bytes / 1048576).toFixed(2)} MB; commit ${t.commit.toFixed(0)} ms, first frame ${t.frame.toFixed(0)} ms)`);
    await o.page.evaluate(() => {
      const s = document.querySelector("[data-file-preview]");
      s.scrollTop = s.scrollHeight;
    });
    const last = await o.page
      .waitForFunction(() => document.querySelector("[data-nb-cell]:last-child")?.hasAttribute("data-nb-drawn"), null, { timeout: 5_000 })
      .then(() => true, () => false);
    if (!last) fail("big: the last cell was never drawn after scrolling to it");
    else ok("big: the last cell is drawn once scrolled to");
    errors.push(...o.errors);
    await o.page.close();
  }

  // 11. errors
  if (errors.length) errors.forEach((e) => fail(`page error: ${e}`));
  else ok("no page errors");
} catch (e) {
  fail(`gate crashed: ${e?.stack ?? e}`);
} finally {
  await browser?.close();
  vite?.kill();
}

console.log(failures ? `\ncheck-nb: ${failures} failure(s)` : "\ncheck-nb: all good");
process.exit(failures ? 1 : 0);
