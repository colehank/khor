// The markdown gate (lane md-render): what the one renderer
// (`components/Markdown`) must and must not let through, on both faces,
// measured in a real engine on the lab page (`lab/md.html`) with nothing
// else of the app around it.
//
// Claims — each a number a machine reads, none a judgment of looks:
//   0. probe: the lab booted, and the long reply really is long;
//   1. a conversation's raw HTML stays text: a `<script>`, an `onerror`,
//      an `<iframe>`, a `javascript:` link — no element, no `on*`
//      attribute, no such address, and the words come back as written;
//   2. a file's raw HTML goes through GitHub's allow-list: the same
//      attacks leave nothing, while `<details>`, `<kbd>`, `<sup>`,
//      `<p align=center>` and a picture beside the file (read from its
//      machine, inlined as `data:`) do become elements; an `https`
//      picture is an element with its address, an `http` one is not (no
//      `img` with an `http:` src, no `srcset`; its alt instead);
//   3. a conversation does not read 「$5 和 $10」 as math and does read
//      `$$…$$` and `\(…\)`; a file reads `$…$`;
//   3b. a conversation's single `$` follows pandoc's rule (currency,
//      ranges, `\$`, inline, fenced and indented code stay text; `$x^2$`
//      is math, also mid-stream);
//   4. a block that has closed is not drawn again: appending to a
//      finished 50 000-character turn redraws at most two of the runs it
//      is drawn in, and a whole streamed turn draws at most one run per
//      chunk plus two per run;
//   5. cutting the growing text tail-first agrees with cutting it whole,
//      at every prefix of the long reply;
//   6. a half-written `**`, `` ` `` or link never shows its markers;
//   7. the heavy work waits for the block to close: an open fence is not
//      coloured and an open mermaid fence is not drawn; closed, both are
//      — and the drawn diagram has no script and no `on*` attribute; a
//      diagram that asks for the `elk` layout (not shipped) stays its
//      source;
//   8. alerts, footnotes, anchors, source lines, table alignment and a
//      list's start number reach the DOM;
//   9. zero page errors.
//
// Run: `npm run md` (boots its own vite; Playwright's WebKit when
// `PLAYWRIGHT_BROWSERS_PATH` points at one, else system Chrome; no cargo
// lock).
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

import { chromium, webkit } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const PORT = Number(process.env.MD_PORT ?? 1485);
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

let failures = 0;
const fail = (msg) => {
  failures += 1;
  console.error(`✗ ${msg}`);
};
const ok = (msg) => console.log(`✓ ${msg}`);

/** A 1×1 picture, what the routed `/api/preview` answers for a file
    picture that exists. */
const PNG = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

const EVIL = [
  "<script>alert(1)</script>",
  '<img src=x onerror="alert(2)">',
  '<iframe src="https://example.com"></iframe>',
  '<a href="javascript:alert(3)">a</a>',
  "[b](javascript:alert(4))",
  '<details ontoggle="alert(5)" open><summary>s</summary>t</details>',
  '<svg onload="alert(6)"><circle r="1"/></svg>',
  "<style>body{display:none}</style>",
  '<form action="https://example.com"><input name="q"></form>',
].join("\n\n");

/** Everything inside the rendered text that an attack would leave. */
const leaks = (selector = "[data-md]") => {
  const root = document.querySelector(selector);
  if (!root) return { missing: true };
  const els = [...root.querySelectorAll("*")];
  return {
    tags: root.querySelectorAll("script, iframe, object, embed, style, form, svg:not([data-mermaid] svg)").length,
    on: els.filter((e) => [...e.attributes].some((a) => /^on/i.test(a.name))).map((e) => e.tagName),
    js: els.filter((e) => [...e.attributes].some((a) => /^\s*javascript:/i.test(a.value))).map((e) => e.tagName),
    text: root.textContent,
  };
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
  const page = await browser.newPage({ viewport: { width: 900, height: 700 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  // An `https` picture is fetched by the page; answered here, so the
  // gate never reaches the network.
  await page.route("https://example.com/**", (route) =>
    route.fulfill({ contentType: "image/png", body: Buffer.from(PNG, "base64") }),
  );
  await page.route("**/api/preview", async (route) => {
    const { path } = JSON.parse(route.request().postData() ?? "{}");
    if (String(path).endsWith("/missing.png")) return route.fulfill({ status: 500, body: "not there" });
    return route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ kind: "image", total: 68, media: "image/png", data: PNG, cut: false }),
    });
  });
  await page.goto(`http://127.0.0.1:${PORT}/lab/md.html?light`);
  await page.waitForFunction(() => typeof window.mdShow === "function", null, { timeout: 60_000 });
  // The first pass through each lazy chunk can make vite re-optimise and
  // reload; touch each once so the measured passes run on a settled page.
  const warm = async () => {
    await page.evaluate(() =>
      window.mdShow({
        text: "$$a$$ :rocket:\n\n```mermaid\ngraph TD; A-->B;\n```\n\n```mermaid\n---\nconfig:\n  layout: elk\n---\nflowchart TD\n  A-->B\n```\n",
      }),
    );
    await page.evaluate(() => window.mdShow({ text: "<kbd>k</kbd> $x$", file: { machine: "m", path: "/r/README.md" } }));
    await wait(1500);
    await page.waitForFunction(() => typeof window.mdShow === "function", null, { timeout: 60_000 });
  };
  await warm();
  await warm();
  const show = async (text, file) => {
    await page.evaluate(([t, f]) => window.mdShow(f ? { text: t, file: f } : { text: t }), [text, file ?? null]);
    // Lazy chunks land a frame or a few later.
    await wait(250);
    await page.evaluate(() => window.mdShow === undefined || new Promise((r) => requestAnimationFrame(() => setTimeout(r, 50))));
  };
  const FILE = { machine: "m", path: "/repo/docs/README.md" };

  // 0. probe
  const long = await page.evaluate(() => window.mdLong());
  if (long.length < 50_000) fail(`probe: the long reply is ${long.length} characters, not 50 000`);
  else ok(`probe: long reply ${long.length} characters`);

  // 1. conversation: raw HTML stays text
  {
    await show(EVIL);
    const l = await page.evaluate(leaks);
    if (l.missing) fail("chat: nothing was rendered");
    else {
      if (l.tags !== 0) fail(`chat: ${l.tags} dangerous element(s) rendered`);
      if (l.on.length) fail(`chat: on* attributes on ${l.on.join(", ")}`);
      if (l.js.length) fail(`chat: javascript: addresses on ${l.js.join(", ")}`);
      const img = await page.evaluate(() => document.querySelectorAll("[data-md] img, [data-md] details").length);
      if (img !== 0) fail(`chat: raw <img>/<details> became ${img} element(s)`);
      if (!l.text.includes("<script>alert(1)</script>")) fail("chat: the script tag did not come back as its own text");
      if (!l.tags && !l.on.length && !l.js.length && !img && l.text.includes("<script>")) ok("chat: raw HTML is text");
    }
  }

  // 2. file: GitHub's allow-list
  {
    const doc = [
      EVIL,
      '<p align="center"><img src="../logo.png" alt="logo" width="32"></p>',
      "按 <kbd>⌘</kbd> 键，x<sup>2</sup>",
      "<details><summary>展开</summary>\n\n里面\n\n</details>",
      "![徽章](https://example.com/badge.png) ![明文](http://example.com/plain.png) ![不在](missing.png)",
      '<img src="http://example.com/raw.png" alt="明文二">',
      '<picture><source srcset="http://example.com/s.png"><img src="https://example.com/p.png" alt="p"></picture>',
    ].join("\n\n");
    const was = failures;
    await show(doc, FILE);
    await wait(300);
    const l = await page.evaluate(leaks);
    const good = await page.evaluate(() => {
      const r = document.querySelector("[data-md]");
      return {
        details: r.querySelectorAll("details > summary").length,
        kbd: r.querySelectorAll("kbd").length,
        sup: r.querySelectorAll("sup").length,
        center: r.querySelectorAll('p[align="center"]').length,
        local: [...r.querySelectorAll("img")].filter((i) => i.src.startsWith("data:image/png")).length,
        https: [...r.querySelectorAll("img")].filter((i) => i.getAttribute("src")?.startsWith("https://example.com/")).length,
        http: [...r.querySelectorAll("img")].filter((i) => /^http:/i.test(i.getAttribute("src") ?? "")).length,
        srcset: r.querySelectorAll("[srcset]").length,
        text: r.textContent,
      };
    });
    if (l.tags !== 0) fail(`file: ${l.tags} dangerous element(s) survived the clean`);
    if (l.on.length) fail(`file: on* attributes on ${l.on.join(", ")}`);
    if (l.js.length) fail(`file: javascript: addresses on ${l.js.join(", ")}`);
    if (good.text.includes("alert(1)")) fail("file: the script's contents were kept as text (GitHub strips them)");
    for (const [k, want] of [["details", 2], ["kbd", 1], ["sup", 1], ["center", 1], ["local", 1]]) {
      if (good[k] < want) fail(`file: ${k} — wanted ${want}, got ${good[k]}`);
    }
    if (good.http !== 0) fail(`file: ${good.http} http picture(s) became <img src>`);
    if (good.srcset !== 0) fail(`file: ${good.srcset} srcset address(es) survived the clean`);
    if (good.https !== 2) fail(`file: ${good.https} https picture(s) drawn, wanted 2`);
    if (!good.text.includes("明文") || !good.text.includes("明文二") || !good.text.includes("不在"))
      fail("file: an http or unreadable picture did not fall back to its alt");
    if (failures === was) ok("file: GitHub's allow-list — attacks gone, details/kbd/sup/align/local and https pictures kept, http ones alt");
  }

  // 3. math
  {
    const count = () => page.evaluate(() => document.querySelectorAll("[data-md] .katex").length);
    await show("价格从 $5 涨到 $10，范围 3~5 天。");
    const dollars = await count();
    const struck = await page.evaluate(() => document.querySelectorAll("[data-md] del").length);
    await show("价格从 $5 涨到 $10。\n\n$$\na^2\n$$\n\n行内 \\(x+y\\) 和 $$z$$。");
    const doubled = await count();
    await show("质能 $E=mc^2$，以及\n\n```math\n\\frac{1}{2}\n```", FILE);
    const file = await count();
    if (dollars !== 0) fail(`math: 「$5 和 $10」 drew ${dollars} formula(s) in a conversation`);
    if (struck !== 0) fail("math: 「3~5」 was struck through in a conversation");
    if (doubled !== 3) fail(`math: $$…$$, \\(…\\) and inline $$…$$ drew ${doubled} formula(s), wanted 3`);
    if (file !== 2) fail(`math: a file's $…$ and \`\`\`math drew ${file}, wanted 2`);
    if (!dollars && !struck && doubled === 3 && file === 2) ok("math: conversation needs $$ or \\( \\); file reads $ — 0 / 3 / 2");
  }

  // 3b. a conversation's single `$` follows pandoc (tex_math_dollars):
  // an opening `$` has a non-space on its right, a closing one a
  // non-space on its left and no digit after it; `\$` is a dollar; code
  // is code. The owner's ruling, 2026-10-19.
  {
    const was = failures;
    const formulas = () =>
      page.evaluate(() => [...document.querySelectorAll("[data-md] .katex-mathml annotation")].map((a) => a.textContent));
    const errors = () => page.evaluate(() => document.querySelectorAll("[data-md] .katex-error").length);
    const plain = ["$5 和 $10", "$5-$10", "costs $5, $10 and $20", "\\$x\\$", "代码 `$x$` 里", "3~5 天 $100"];
    for (const t of plain) {
      await show(t);
      const f = await formulas();
      if (f.length) fail(`pandoc: 「${t}」 drew ${JSON.stringify(f)}, wanted text`);
      // And reads as written: an escape this build added never shows.
      const seen = await page.evaluate(() => document.querySelector("[data-md]").textContent);
      const want = t.replace(/\\\$/g, "$").replace(/`/g, "");
      if (seen !== want) fail(`pandoc: 「${t}」 reads 「${seen}」, wanted 「${want}」`);
    }
    const math = [
      ["$x^2$", ["x^2"]],
      ["$2x+1$", ["2x+1"]],
      ["$\\alpha$ 和 $\\beta$", ["\\alpha", "\\beta"]],
      ["句末 $E=mc^2$。", ["E=mc^2"]],
    ];
    for (const [t, want] of math) {
      await show(t);
      const f = await formulas();
      if (JSON.stringify(f) !== JSON.stringify(want)) fail(`pandoc: 「${t}」 drew ${JSON.stringify(f)}, wanted ${JSON.stringify(want)}`);
    }
    // Indented code (a pasted log) is code: its dollars print as written.
    await show("日志：\n\n    echo $x$ $5\n    total $10\n\n之后。");
    const log = await page.evaluate(() => ({
      code: document.querySelector("[data-md] pre code")?.textContent ?? null,
      katex: document.querySelectorAll("[data-md] .katex").length,
    }));
    if (log.code !== "echo $x$ $5\ntotal $10\n" || log.katex) fail(`pandoc: indented code reads ${JSON.stringify(log.code)} with ${log.katex} formula(s)`);
    // Character by character, the way a turn drips: never an error, and
    // never a formula that is not the one being written.
    const drip = "$x^2$ 之后";
    for (let i = 1; i <= drip.length; i++) {
      await page.evaluate((t) => window.mdShow({ text: t }), drip.slice(0, i));
      const f = await formulas();
      const e = await errors();
      if (e || f.some((x) => x !== "x^2")) fail(`pandoc: streaming 「${drip.slice(0, i)}」 drew ${JSON.stringify(f)} (${e} error(s))`);
    }
    if (JSON.stringify(await formulas()) !== JSON.stringify(["x^2"])) fail("pandoc: the dripped 「$x^2$ 之后」 ended without its formula");
    if (failures === was) ok("pandoc: currency, ranges, \\$ and code stay text; $x^2$ and friends are math, also while dripping");
  }

  // 4. a closed block is not drawn again
  {
    await show("");
    const zero = await page.evaluate(() => window.mdMeasured().blocks);
    await show(long);
    const blocks = await page.evaluate(() => document.querySelectorAll("[data-md] > *").length);
    const before = await page.evaluate(() => window.mdMeasured().blocks);
    // Not by vacuum: the counter has to have counted the turn's blocks.
    const runs = before - zero;
    if (runs < 10) fail(`redraw: probe dead — drawing the long reply counted ${runs} blocks`);
    await show(long + "又一句。");
    const appended = (await page.evaluate(() => window.mdMeasured().blocks)) - before;
    const s0 = await page.evaluate(() => window.mdMeasured().blocks);
    const run = await page.evaluate((t) => window.mdStream({ text: t, step: 400 }), long);
    const streamed = (await page.evaluate(() => window.mdMeasured().blocks)) - s0;
    // A regular expression is not a footnote: `[^a-z]` in a code block
    // must not send the whole turn back to one block.
    const regex = long + "\n\n```js\nconst re = /[^a-z]/;\n```\n\n";
    await show("");
    const r0 = await page.evaluate(() => window.mdMeasured().blocks);
    await show(regex);
    const regexRuns = (await page.evaluate(() => window.mdMeasured().blocks)) - r0;
    if (regexRuns < runs) fail(`redraw: a [^a-z] in code drew the turn as ${regexRuns} block(s), not ${runs}+`);
    const bound = run.chunks + 2 * runs;
    if (appended > 2) fail(`redraw: appending to a finished turn drew ${appended} blocks, wanted ≤ 2`);
    if (streamed > bound) fail(`redraw: streaming drew ${streamed} blocks over ${run.chunks} chunks, wanted ≤ ${bound}`);
    if (blocks < 100) fail(`redraw: the long reply rendered as ${blocks} top-level elements`);
    if (appended <= 2 && streamed <= bound && blocks >= 100 && runs >= 10)
      ok(`redraw: append redraws ${appended}; ${run.chunks} chunks drew ${streamed} blocks (≤ ${bound})`);
  }

  // 5. tail-first cutting agrees with cutting whole
  {
    const bad = await page.evaluate((t) => window.mdCutAgrees(t, 97), long);
    if (bad.length) fail(`cut: ${bad.length} prefixes cut differently tail-first, first at ${bad[0]}`);
    else ok(`cut: tail-first agrees with whole at every 97-character prefix`);
  }

  // 6. half-written markers never show
  {
    const cases = [
      ["这是 **加粗的", "**", "strong"],
      ["看这里 `code", "`", "code"],
      ["打开 [链接](https://exa", "](", null],
      ["斜体 *开始", "*", "em"],
    ];
    let bad = 0;
    for (const [text, marker, el] of cases) {
      await show(text);
      const r = await page.evaluate(
        ([m, e]) => {
          const root = document.querySelector("[data-md]");
          return { shows: root.textContent.includes(m), has: e === null || root.querySelector(e) !== null };
        },
        [marker, el],
      );
      if (r.shows || !r.has) {
        bad += 1;
        fail(`mend: 「${text}」 shows 「${marker}」 or has no <${el}>`);
      }
    }
    if (!bad) ok("mend: half-written **, `, [](, * never show their markers");
  }

  // 7. heavy work waits for the close
  {
    const shiki = () => page.evaluate(() => document.querySelectorAll('[data-md] span[style*="--shiki"]').length);
    const drawn = () => page.evaluate(() => document.querySelectorAll('[data-md] [data-mermaid="drawn"] svg').length);
    const p0 = await page.evaluate(() => window.mdMeasured().paints);
    await show("看：\n\n```ts\nconst a: number = 1;\n");
    const openSpans = await shiki();
    const p1 = await page.evaluate(() => window.mdMeasured().paints);
    await show("看：\n\n```ts\nconst a: number = 1;\n```\n");
    await wait(500);
    const closedSpans = await shiki();
    await show("图：\n\n```mermaid\ngraph TD; A-->B;\n");
    await wait(800);
    const openDrawn = await drawn();
    await show('图：\n\n```mermaid\ngraph TD; A["<img src=x onerror=alert(7)>一<br>二"]-->B;\n```\n');
    for (let i = 0; i < 40 && (await drawn()) === 0; i++) await wait(250);
    const closedDrawn = await drawn();
    const l = await page.evaluate(leaks);
    if (openSpans !== 0 || p1 !== p0) fail(`heavy: an open fence was coloured (${openSpans} spans, ${p1 - p0} paints)`);
    if (closedSpans === 0) fail("heavy: a closed fence was never coloured");
    if (openDrawn !== 0) fail("heavy: an open mermaid fence was drawn");
    if (closedDrawn !== 1) fail(`heavy: a closed mermaid fence drew ${closedDrawn} diagram(s), wanted 1`);
    if (l.on.length || l.js.length) fail(`heavy: the diagram carries on*/javascript: (${[...l.on, ...l.js].join(", ")})`);
    if (await page.evaluate(() => document.querySelectorAll("[data-md] img").length)) fail("heavy: a label's <img> became an element");
    // `layout: elk` is not in this build: the diagram stays its source.
    await show("图：\n\n```mermaid\n---\nconfig:\n  layout: elk\n---\nflowchart TD\n  A-->B\n```\n");
    await wait(2500);
    const elk = await page.evaluate(() => ({
      drawn: document.querySelectorAll('[data-md] [data-mermaid="drawn"]').length,
      source: document.querySelector('[data-md] [data-mermaid="source"]')?.textContent ?? "",
    }));
    if (elk.drawn !== 0 || !elk.source.includes("layout: elk")) fail(`heavy: an elk diagram drew ${elk.drawn} / showed 「${elk.source.slice(0, 40)}」, wanted its source`);
    await show('图：\n\n```mermaid\ngraph TD; A["<img src=x onerror=alert(7)>一<br>二"]-->B;\n```\n');
    for (let i = 0; i < 40 && (await drawn()) === 0; i++) await wait(250);
    const label = await page.evaluate(() => document.querySelector('[data-md] [data-mermaid="drawn"]')?.textContent ?? "");
    if (closedDrawn && !(label.includes("一") && label.includes("二"))) fail(`heavy: the diagram lost its two-line label (${JSON.stringify(label)})`);
    if (!openSpans && p1 === p0 && closedSpans && !openDrawn && closedDrawn === 1 && !l.on.length)
      ok("heavy: open fences plain, closed ones coloured and drawn, the diagram clean");
  }

  // 8. GitHub's parts reach the DOM
  {
    const doc = [
      "# 标题",
      "",
      "> [!WARNING]",
      "> 小心。",
      "",
      "> [!NOTE] 同一行",
      "",
      "## 安装",
      "",
      "跳到 [安装](#安装)，脚注[^1]。",
      "",
      "| 左 | 中 | 右 |",
      "|:--|:-:|--:|",
      "| a | b | c |",
      "",
      "3. 三",
      "4. 四",
      "",
      "- [x] 好了",
      "",
      "[^1]: 注。",
    ].join("\n");
    await show(doc, FILE);
    const r = await page.evaluate(() => {
      const root = document.querySelector("[data-md]");
      const h = [...root.querySelectorAll("h2")].find((e) => e.textContent === "安装");
      const ths = [...root.querySelectorAll("th")].map((e) => getComputedStyle(e).textAlign);
      return {
        warning: root.querySelectorAll('[data-alert="warning"] [data-alert-title]').length,
        notAlert: [...root.querySelectorAll("blockquote")].some((b) => b.textContent.includes("[!NOTE] 同一行")),
        id: h?.id ?? null,
        line: h?.getAttribute("data-src-line") ?? null,
        footnotes: root.querySelectorAll("[data-footnotes] li[id]").length,
        ref: root.querySelectorAll('[data-md-link^="#"][id]').length,
        ths,
        start: root.querySelector("ol")?.getAttribute("start") ?? null,
        task: root.querySelectorAll('.task-list-item input[type="checkbox"][disabled]').length,
      };
    });
    const want = { warning: 1, notAlert: true, id: "安装", line: "8", footnotes: 1, ths: "left,center,right", start: "3", task: 1 };
    const got = { ...r, ths: r.ths.join(",") };
    let bad = 0;
    for (const [k, v] of Object.entries(want)) {
      if (got[k] !== v) {
        bad += 1;
        fail(`github: ${k} — wanted ${JSON.stringify(v)}, got ${JSON.stringify(got[k])}`);
      }
    }
    if (r.ref < 1) {
      bad += 1;
      fail("github: the footnote reference has no id to come back to");
    }
    // The anchor moves within the text.
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.evaluate(() => {
      const pad = document.createElement("div");
      pad.style.height = "3000px";
      document.querySelector("[data-lab-md]").append(pad);
    });
    await page.evaluate(() => document.querySelector('[data-md-link="#%E5%AE%89%E8%A3%85"], [data-md-link="#安装"]')?.click());
    await wait(200);
    const top = await page.evaluate(() => document.getElementById("安装")?.getBoundingClientRect().top ?? null);
    if (top === null || Math.abs(top) > 2) {
      bad += 1;
      fail(`github: following #安装 left the heading at ${top}, wanted 0`);
    }
    if (!bad) ok("github: alert, anchor (and following it), footnotes, source line, alignment, start, task box");
  }

  // 9. no page errors
  if (errors.length) fail(`page errors: ${errors.join(" | ")}`);
  else ok("no page errors");
} catch (e) {
  fail(String(e?.stack ?? e));
} finally {
  await browser?.close();
  vite?.kill();
}

if (failures) {
  console.error(`\ncheck-md: ${failures} failure(s)`);
  process.exit(1);
}
console.log("\ncheck-md: all good");
