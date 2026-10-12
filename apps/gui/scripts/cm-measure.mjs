// Lane cm-d's measures (CM-D-BRIEF §Measure), headless Chrome only.
//
//   node scripts/cm-measure.mjs commit <rev> [runs]
//       one commit's page (`lab/commit.html`, a production build) at 390
//       and 1400: ms from the page's first render to every text file's
//       diff drawn, to the syntax ink settled, and the JS heap after.
//       `git_commit` is answered from this checkout's own git, in the
//       shape the node answers today (texts on both sides; `CM_SHAPE=
//       hunks` for the answer before cm-d, which the old page reads).
//   node scripts/cm-measure.mjs shots <rev> <dir> [theme]
//       the same page at 390 and 1400 into <dir>: as it opens, and with
//       the first folded stretch opened; says whether any file's view
//       scrolls on its own (it must not: the page is one scroll).
//   node scripts/cm-measure.mjs screens <dir> [theme]
//       `lab/cmd.html`'s screens at 390 and 1400, each driven and judged
//       (exit 1 on a wrong answer), shot into <dir>.
//   node scripts/cm-measure.mjs big [lines]
//       a generated `.ts` of that many lines (a function every 10) in
//       the editor (`lab/big.html`): ms to open, to fold all, and per
//       selection change with a three-letter word selected.
//
// Out: JSON on stdout, one line per measure. `CM_OUT` a directory for
// the build (default the lane's target, never the system disk).
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
import { build, preview } from "vite";

const gui = fileURLToPath(new URL("..", import.meta.url));
const repo = join(gui, "../..");
const out = process.env.CM_OUT ?? join(repo, "target/cm-measure");
const [what = "commit", arg, runsArg] = process.argv.slice(2);
const runs = Number(runsArg ?? 5);

const git = (...a) => execFileSync("git", ["-C", repo, ...a], { maxBuffer: 1 << 28 });
const CAP = 256 * 1024;

/** The node's answer for `rev`, rebuilt from git's own CLI: first
    parent, renames at git's default, both sides' bytes or the hunks. */
function commitAnswer(rev, shape) {
  const parent = git("rev-parse", `${rev}^1`).toString().trim();
  const rows = git("diff", "--name-status", "-M", parent, rev)
    .toString()
    .trim()
    .split("\n")
    .map((l) => l.split("\t"));
  const numstat = new Map(
    git("diff", "--numstat", "-M", parent, rev)
      .toString()
      .trim()
      .split("\n")
      .map((l) => l.split("\t"))
      .map(([a, r, p]) => [p.includes(" => ") ? p.replace(/\{?([^{}]*) => ([^{}]*)\}?/, "$2").replace("//", "/") : p, [Number(a) || 0, Number(r) || 0]]),
  );
  const files = rows.map(([s, a, b]) => {
    const status = s[0];
    const path = b ?? a;
    const from = status === "R" ? a : "";
    const was = status === "A" ? Buffer.alloc(0) : git("show", `${parent}:${from || path}`);
    const now = status === "D" ? Buffer.alloc(0) : git("show", `${rev}:${path}`);
    const binary = [was, now].some((x) => x.subarray(0, 8000).includes(0));
    const [added, removed] = binary ? [0, 0] : (numstat.get(path) ?? [0, 0]);
    const f = { path, from, status, added, removed, binary };
    if (shape === "hunks") {
      const d = binary ? "" : git("diff", "-U3", "--no-color", "-M", parent, rev, "--", ...(from ? [from, path] : [path])).toString();
      f.hunks = [];
      let h = null;
      for (const l of d.split("\n")) {
        const m = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(l);
        if (m) f.hunks.push((h = { old_at: Number(m[1]), new_at: Number(m[2]), lines: [] }));
        else if (h && /^[ +-]/.test(l) && !/^(\+\+\+|---) /.test(l)) h.lines.push(l);
      }
    } else {
      const big = !binary && (was.length > CAP || now.length > CAP);
      Object.assign(f, { big, before: binary || big ? "" : was.toString(), after: binary || big ? "" : now.toString() });
    }
    return f;
  });
  const c = git("show", "-s", "--format=%H%x00%an%x00%ct%x00%s", rev).toString().trim().split("\0");
  return {
    commit: { id: c[0], author: c[1], when_ms: Number(c[2]) * 1000, subject: c[3], body: "" },
    files,
    total: files.length,
    more: false,
  };
}

const median = (xs) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)];

async function serve(page) {
  await build({
    root: gui,
    configFile: join(gui, "vite.config.ts"),
    logLevel: "warn",
    build: { outDir: out, emptyOutDir: true, rollupOptions: { input: { [page]: join(gui, `lab/${page}.html`) } } },
  });
  return preview({ root: gui, configFile: join(gui, "vite.config.ts"), logLevel: "warn", build: { outDir: out }, preview: { port: 0, strictPort: false } });
}

async function heap(page) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("HeapProfiler.collectGarbage");
  await cdp.send("Performance.enable");
  const { metrics } = await cdp.send("Performance.getMetrics");
  return metrics.find((m) => m.name === "JSHeapUsedSize").value;
}

const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server"] });
try {
  if (what === "commit") {
    const rev = git("rev-parse", arg ?? "HEAD").toString().trim();
    const shape = process.env.CM_SHAPE === "hunks" ? "hunks" : "texts";
    const answer = JSON.stringify(commitAnswer(rev, shape));
    const server = await serve("commit");
    const base = server.resolvedUrls.local[0];
    for (const width of [390, 1400]) {
      const first = [];
      const firsts = [];
      const chunks = [];
      const ink = [];
      const heaps = [];
      for (let i = 0; i < runs; i += 1) {
        const page = await browser.newPage({ viewport: { width, height: 844 } });
        if (process.env.CM_LOG) page.on("console", (m) => m.text().startsWith("CMD") && console.log(m.text()));
        await page.route("**/api/git_commit", (r) => r.fulfill({ contentType: "application/json", body: answer }));
        await page.goto(`${base}lab/commit.html?light&id=${rev}`);
        const t = await page.evaluate(
          () =>
            new Promise((done) => {
              const t0 = window.__t0;
              // When the diff's lazy chunk had arrived, if it is one.
              const chunkAt = () => {
                const r = performance.getEntriesByType("resource").find((e) => /CommitDiff-/.test(e.name));
                return r ? Math.round(r.responseEnd - t0) : null;
              };
              let drawn = null;
              let firstView = null;
              let inked = null;
              let lastInk = -1;
              const tick = () => {
                const files = [...document.querySelectorAll("[data-commit-file]")];
                if (files.length > 0) {
                  // A text file's diff is drawn when its surface is: the
                  // hunks' rows before cm-d, a CodeMirror view after.
                  const text = files.filter((f) => !f.querySelector("[data-commit-binary],[data-commit-big]") && !/不是文本文件|Not text/.test(f.textContent));
                  const shown = text.filter((f) => f.querySelector(".cm-editor .cm-line, [data-diff-line]"));
                  if (firstView === null && shown.length > 0) firstView = performance.now() - t0;
                  if (drawn === null && shown.length === text.length) drawn = performance.now() - t0;
                  const now = document.querySelectorAll('[data-commit-file] [class*="tok-"]').length;
                  if (now !== lastInk) {
                    lastInk = now;
                    inked = performance.now() - t0;
                  }
                }
                if (drawn !== null && performance.now() - t0 - inked > 1500) return done({ drawn, firstView, inked: Math.max(drawn, inked), chunk: chunkAt() });
                if (performance.now() - t0 > 20_000) return done({ drawn, firstView, inked, chunk: chunkAt() });
                requestAnimationFrame(tick);
              };
              requestAnimationFrame(tick);
            }),
        );
        first.push(t.drawn);
        firsts.push(t.firstView);
        chunks.push(t.chunk ?? 0);
        ink.push(t.inked);
        heaps.push(await heap(page));
        await page.close();
      }
      console.log(
        JSON.stringify({
          measure: "commit",
          rev: rev.slice(0, 8),
          shape,
          width,
          files: JSON.parse(answer).files.length,
          answerKB: Math.round(answer.length / 1024),
          firstFileMs: Math.round(median(firsts)),
          drawnMs: Math.round(median(first)),
          chunkMs: Math.round(median(chunks)),
          inkMs: Math.round(median(ink)),
          heapMB: +(median(heaps) / 1048576).toFixed(1),
          runs,
        }),
      );
    }
    await server.close();
  } else if (what === "shots") {
    const rev = git("rev-parse", arg).toString().trim();
    const dir = runsArg;
    const theme = process.argv[5] ?? "light";
    const answer = JSON.stringify(commitAnswer(rev, "texts"));
    const server = await serve("commit");
    const base = server.resolvedUrls.local[0];
    for (const width of [390, 1400]) {
      const page = await browser.newPage({ viewport: { width, height: 844 }, deviceScaleFactor: 2 });
      await page.route("**/api/git_commit", (r) => r.fulfill({ contentType: "application/json", body: answer }));
      const subject = encodeURIComponent(git("show", "-s", "--format=%s", rev).toString().trim());
      await page.goto(`${base}lab/commit.html?${theme}&id=${rev}&subject=${subject}`);
      await page.waitForSelector("[data-commit-file] .cm-editor");
      await page.waitForTimeout(1500);
      const scrollers = await page.evaluate(() => ({
        views: document.querySelectorAll("[data-commit-file] .cm-editor").length,
        nested: [...document.querySelectorAll("[data-commit-file] .cm-scroller")].filter((s) => s.scrollHeight > s.clientHeight + 1).length,
        ink: document.querySelectorAll('[data-commit-file] [class*="tok-"]').length,
        folds: document.querySelectorAll("[data-commit-file] .cm-collapsedLines").length,
        page: (() => {
          const b = document.querySelector("[data-commit-scroll]");
          return { scrollH: b.scrollHeight, clientH: b.clientHeight, wider: b.scrollWidth - b.clientWidth };
        })(),
      }));
      console.log(JSON.stringify({ shots: width, ...scrollers }));
      await page.screenshot({ path: join(dir, `commit-${theme}-${width}.png`) });
      // The first folded stretch, opened and brought to the top.
      const bar = page.locator("[data-commit-file] .cm-collapsedLines").first();
      await bar.scrollIntoViewIfNeeded();
      await bar.click();
      await page.waitForTimeout(400);
      await page.evaluate(() => {
        const b = document.querySelector("[data-commit-scroll]");
        const f = document.querySelector("[data-commit-file]");
        b.scrollTop += f.getBoundingClientRect().top - b.getBoundingClientRect().top;
        // The press scrolled the file sideways to the bar's middle.
        for (const s of document.querySelectorAll("[data-commit-file] .cm-scroller")) s.scrollLeft = 0;
      });
      await page.waitForTimeout(400);
      await page.screenshot({ path: join(dir, `commit-${theme}-${width}-opened.png`) });
      await page.close();
    }
    await server.close();
  } else if (what === "screens") {
    const dir = arg;
    const theme = runsArg && Number.isNaN(Number(runsArg)) ? runsArg : "light";
    const server = await serve("cmd");
    const base = server.resolvedUrls.local[0];
    let bad = 0;
    const judge = (ok, what) => {
      if (!ok) bad += 1;
      console.log(`${ok ? "  ok " : "  BAD"} ${what}`);
    };
    for (const width of [390, 1400]) {
      const open = async (screen) => {
        const page = await browser.newPage({ viewport: { width, height: 844 }, deviceScaleFactor: 2 });
        await page.goto(`${base}lab/cmd.html?${theme}#${screen}`);
        await page.waitForFunction(() => window.__view && document.querySelector('[data-code-view] [class*="tok-"]'));
        return page;
      };
      // Item 2: Tab in a 4-space .rs inserts four spaces, Enter keeps them.
      {
        const page = await open("rs");
        await page.evaluate(() => {
          const v = window.__view;
          v.focus();
          const l = v.state.doc.line(6);
          v.dispatch({ selection: { anchor: l.to }, userEvent: "select" });
        });
        await page.keyboard.press("Enter");
        await page.keyboard.press("Tab");
        await page.keyboard.type("// one deeper");
        const r = await page.evaluate(() => {
          const v = window.__view;
          return { line: v.state.doc.line(7).text };
        });
        judge(r.line === "        // one deeper", `${width} rs: Enter then Tab after a 4-space line reads ${JSON.stringify(r.line)} — eight spaces`);
        await page.screenshot({ path: join(dir, `rs-tab-${theme}-${width}.png`) });
        await page.close();
      }
      // Item 3: the caret's line, the selected word's other places, the
      // whole-word toggle, and the last line able to reach the top.
      {
        const page = await open("ts");
        const r = await page.evaluate(() => {
          const v = window.__view;
          v.focus();
          // `draft` on line 6 (the parameter), selected.
          const l = v.state.doc.line(6);
          const at = l.from + l.text.indexOf("draft");
          v.dispatch({ selection: { anchor: at, head: at + 5 }, userEvent: "select" });
          const one = (sel) => [...document.querySelectorAll(sel)].map((e) => e.textContent);
          return { active: one(".cm-activeLine"), gutter: one(".cm-lineNumbers .cm-activeLineGutter"), matches: one(".cm-selectionMatch") };
        });
        judge(r.active.length === 1 && r.active[0].includes("reread(") && r.gutter.join() === "6", `${width} ts: the caret's line ${JSON.stringify(r.active)}, its number ${JSON.stringify(r.gutter)} — line 6`);
        // `draft` stands three more times as a word (lines 8 ×2, … ) and
        // never inside `Draft` (case) — whole words, case as typed.
        judge(r.matches.length >= 2 && r.matches.every((m) => m === "draft"), `${width} ts: the selection's other places ${JSON.stringify(r.matches)} — every other \`draft\``);
        await page.screenshot({ path: join(dir, `ts-active-matches-${theme}-${width}.png`) });
        // One letter selected says nothing (minSelectionLength 2).
        const lone = await page.evaluate(() => {
          const v = window.__view;
          const l = v.state.doc.line(6);
          v.dispatch({ selection: { anchor: l.from + 7, head: l.from + 8 }, userEvent: "select" });
          return document.querySelectorAll(".cm-selectionMatch").length;
        });
        judge(lone === 0, `${width} ts: one letter selected marks ${lone} other places — none`);
        // ⌘F, `text`: seven without whole words, five with — the two
        // inside `fetchFileText` drop out.
        await page.evaluate(() => {
          const v = window.__view;
          v.dispatch({ selection: { anchor: 0 } });
          document.querySelector("[data-code-view]").__find();
        });
        await page.waitForSelector("[data-code-find-input]");
        await page.fill("[data-code-find-input]", "text");
        await page.waitForTimeout(150);
        const loose = await page.locator("[data-code-find-hits]").textContent();
        await page.locator("[data-code-find-word]").click();
        await page.waitForTimeout(150);
        const whole = await page.locator("[data-code-find-hits]").textContent();
        const pressed = await page.locator("[data-code-find-word]").getAttribute("aria-pressed");
        const total = (t) => Number(t.split("/").pop());
        judge(total(loose) === 7 && total(whole) === 5 && pressed === "true", `${width} ts: 「text」 finds ${loose} loose, ${whole} as a whole word (pressed ${pressed}) — 7 and 5`);
        await page.screenshot({ path: join(dir, `ts-find-word-${theme}-${width}.png`) });
        // scrollPastEnd: scrolled to its end, the last line stands near the
        // top of the scroller, not at its foot.
        const end = await page.evaluate(async () => {
          const v = window.__view;
          v.scrollDOM.scrollTop = v.scrollDOM.scrollHeight;
          await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
          const last = v.lineBlockAt(v.state.doc.length);
          const top = v.documentTop + last.top - v.scrollDOM.getBoundingClientRect().top;
          return { top: Math.round(top), room: v.scrollDOM.clientHeight };
        });
        judge(end.top < end.room / 3, `${width} ts: at the end the last line stands ${end.top}px from the top of ${end.room} — in the top third`);
        await page.close();
      }
      // Item 4: a TOML file (a stream mode, no tree) folds by indent; a
      // TypeScript file still folds by its tree.
      {
        const page = await open("toml");
        await page.waitForFunction(() => window.__view.state.facet(window.__fold.foldService).length > 0, null, { timeout: 3000 }).catch(() => {});
        const r = await page.evaluate(() => {
          const v = window.__view;
          const { foldable, foldCode, foldService } = window.__fold;
          const at = (n) => {
            const l = v.state.doc.line(n);
            const f = foldable(v.state, l.from, l.to);
            return f && { from: v.state.doc.lineAt(f.from).number, to: v.state.doc.lineAt(f.to).number };
          };
          const out = { services: v.state.facet(foldService).length, section: at(10), list: at(13), flat: at(1), leaf: at(11) };
          v.dispatch({ selection: { anchor: v.state.doc.line(10).from } });
          foldCode(v);
          out.placeholders = document.querySelectorAll(".cm-foldPlaceholder").length;
          out.markers = document.querySelectorAll(".cm-foldGutter [data-fold]").length;
          return out;
        });
        judge(
          r.services === 1 && r.section?.from === 10 && r.section?.to === 16 && r.list?.from === 13 && r.list?.to === 15 && r.flat === null && r.leaf === null && r.placeholders === 1,
          `${width} toml: by indent ${JSON.stringify(r)} — [workspace.metadata.release] folds 10–16, its list 13–15 (the closing bracket stands level with it), a flat section and a leaf nothing; folded once`,
        );
        await page.screenshot({ path: join(dir, `toml-folded-${theme}-${width}.png`) });
        await page.close();
        const ts = await open("ts");
        const t = await ts.evaluate(() => {
          const v = window.__view;
          const { foldable, foldService } = window.__fold;
          const l = v.state.doc.line(6);
          const f = foldable(v.state, l.from, l.to);
          return { services: v.state.facet(foldService).length, to: f && f.to, closer: v.state.doc.line(10).from, indentEnd: v.state.doc.line(9).to };
        });
        judge(t.services === 0 && t.to === t.closer, `${width} ts: ${JSON.stringify(t)} — no indent service; reread() folds by its tree, up to the \`}\``);
        await ts.close();
      }
    }
    await server.close();
    if (bad) process.exitCode = 1;
  } else if (what === "big") {
    const lines = Number(arg ?? 50_000);
    const server = await serve("big");
    const base = server.resolvedUrls.local[0];
    const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
    await page.goto(`${base}lab/big.html?light&lines=${lines}`);
    const r = await page.evaluate(() => window.__big());
    console.log(JSON.stringify({ measure: "big", lines, ...r }));
    await server.close();
  }
} finally {
  await browser.close();
}
