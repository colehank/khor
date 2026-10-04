// The glyph atlas gate (item 24; the owner's screenshot of CJK runs drawn
// as fragments of other glyphs on entering a session, Mac and iPhone).
//
// xterm's WebGL addon shares one glyph atlas among every terminal on a page
// with the same font settings. `src/lib/term-atlas.ts` keeps it honest; this
// gate holds that in a real browser with WebGL, on real xterm, in four
// parts:
//
//   1. **The harness can see the fault.** Without the module, a clear of
//      the shared atlas from one terminal changes what another draws
//      (xterm.js #6014). If it does not, the gate cannot tell a fix from
//      nothing and says so as a dead probe.
//   2. **One pane's clear leaves its sibling as it was**, with the module.
//   3. **A late font**: a terminal draws CJK in a face that is still
//      loading (served 1.5 s late), the face arrives, and each CJK cell
//      must equal a fresh terminal's render in that face.
//   4. **A lost context**: the pane falls back to the DOM renderer at once,
//      WebGL comes back when the page is shown again and draws the same
//      cells as before the loss, and a second loss soon after keeps the
//      DOM renderer.
//
// Needs Chrome (playwright-core's `chrome` channel) and a CJK font file;
// on a machine without one it says so and stops.
import { chromium } from "playwright-core";
import { build } from "esbuild";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const gui = fileURLToPath(new URL("..", import.meta.url));
const FONT = ["/System/Library/Fonts/Supplemental/Arial Unicode.ttf", "/usr/share/fonts/truetype/arphic/uming.ttc"].find(existsSync);
if (!FONT) {
  console.log("atlas gate: no CJK font file on this machine, nothing to load late");
  process.exit(0);
}
let bad = 0;
const fail = (m) => {
  bad++;
  console.error(`atlas gate: ${m}`);
};

const bundle = await build({
  stdin: {
    contents: `
      import { Terminal } from "@xterm/xterm";
      import { WebglAddon } from "@xterm/addon-webgl";
      import * as atlas from "./src/lib/term-atlas.ts";
      window.lab = { Terminal, WebglAddon, atlas };
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

const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const page = await browser.newPage({ viewport: { width: 900, height: 700 }, deviceScaleFactor: 1 });
page.on("pageerror", (e) => fail(`page error: ${e.message}`));
await page.route("**/late.ttf", async (route) => {
  await new Promise((r) => setTimeout(r, 1500));
  await route.fulfill({ body: readFileSync(FONT), contentType: "font/ttf" });
});
await page.route("http://atlas.test/", (route) =>
  route.fulfill({
    contentType: "text/html",
    body: `<!doctype html><meta charset=utf-8><style>${css} body{margin:0;background:#000} .t{width:860px;height:120px;margin:4px}</style>
      <style>@font-face{font-family:LateFace;src:url(/late.ttf);font-display:block;unicode-range:U+3000-303F,U+4E00-9FFF,U+FF00-FFEF}</style>
      <div id=a class=t></div><div id=b class=t></div><div id=c class=t></div><div id=d class=t></div>`,
  }),
);
await page.goto("http://atlas.test/");
await page.addScriptTag({ content: bundle.outputFiles[0].text });

const CJK = "整体界面重做：一张底加灰线，玻璃控件，底部四项导航";
// Another pane's text, other glyphs: what it draws into a cleared atlas
// lands where the first pane's coordinates point.
const OTHER = "终端字号滑杆，光标摇杆震动，历史命令搜索，块菜单拷贝";
const make = (sel, family, guarded, text = CJK) =>
  page.evaluate(
    async ([sel, family, guarded, text]) => {
      const { Terminal, WebglAddon, atlas } = window.lab;
      const term = new Terminal({ fontFamily: family, fontSize: 16, cols: 60, rows: 5, allowProposedApi: true });
      term.open(document.querySelector(sel));
      window.terms ??= {};
      const h = { term, addon: null, fell: [] };
      if (guarded) h.un = atlas.keepWebgl(term, { onFallback: (why) => h.fell.push(why) });
      else {
        h.addon = new WebglAddon();
        term.loadAddon(h.addon);
      }
      window.terms[sel] = h;
      await new Promise((r) => term.write(`${text}\r\n${text}\r\nabc 123\r\n`, r));
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      return !!term.element.querySelector("canvas");
    },
    [sel, family, guarded, text],
  );
/** The first two rows of a terminal, as pixels. */
const shot = async (sel) => {
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
  const box = await page.locator(`${sel} .xterm-screen`).boundingBox();
  return page.screenshot({ clip: { x: box.x, y: box.y, width: Math.min(box.width, 600), height: 40 } });
};
const same = (a, b) => Buffer.compare(a, b) === 0;
/** Something for a terminal to draw, away from the rows compared. */
const poke = (sel) => page.evaluate((sel) => new Promise((r) => window.terms[sel].term.write("x", r)), sel);
const done = (sel) => page.evaluate((sel) => { const h = window.terms[sel]; h.un?.(); h.addon?.dispose(); h.term.dispose(); }, sel);

// 1. The fault, without the module.
if (!(await make("#a", "monospace", false)) || !(await make("#b", "monospace", false, OTHER))) {
  console.error("probe dead: no WebGL canvas in headless Chrome");
  process.exit(1);
}
const a0 = await shot("#a");
await page.evaluate(() => window.terms["#b"].addon.clearTextureAtlas());
await poke("#a");
const a1 = await shot("#a");
const faultSeen = !same(a0, a1);
console.log(`atlas gate: without the module, a sibling's clear ${faultSeen ? "changed" : "did not change"} the other terminal's CJK rows`);
if (!faultSeen) fail("probe dead: the unguarded sibling clear changed nothing, so this gate could not see the fault");
await done("#a");
await done("#b");

// 2. With the module: one pane's clear leaves the other as it was.
await page.evaluate(() => document.fonts.ready);
await make("#a", "monospace", true);
await make("#b", "monospace", true, OTHER);
const g0 = await shot("#a");
// The clear a pane used to make of the shared atlas, on its own.
await page.evaluate(() => window.terms["#b"].term.clearTextureAtlas());
await poke("#a");
if (!same(g0, await shot("#a"))) fail("with the module, a sibling's clear still changed the other terminal's CJK rows");
await page.evaluate(() => window.lab.atlas.clearAtlases());
await poke("#a");
if (!same(g0, await shot("#a"))) fail("clearAtlases() left a terminal drawing something other than it drew before");

// 3. A late font: drawn while loading, then the face arrives.
await make("#c", "LateFace, monospace", true);
await page.evaluate(() => document.fonts.load("16px LateFace", "整"));
await page.evaluate(() => new Promise((r) => setTimeout(r, 300)));
const lateFont = await shot("#c");
// A fresh render: every atlas emptied by hand, so each glyph is drawn
// again now that the face is in. (A second terminal in the same face would
// not do: it shares the atlas, stale rasters and all.)
await page.evaluate(() => window.lab.atlas.clearAtlases());
const fresh = await shot("#c");
if (!same(lateFont, fresh)) fail("a terminal that drew before its face arrived still draws the stand-in's rasters after the face came in");
else console.log("atlas gate: the late face redrew to match a fresh render");

// 4. A lost context, and a second one soon after.
const before = await shot("#a");
const loss = () =>
  page.evaluate(() => {
    for (const canvas of window.terms["#a"].term.element.querySelectorAll("canvas")) {
      const gl = canvas.getContext("webgl2");
      gl?.getExtension("WEBGL_lose_context")?.loseContext();
    }
  });
const onGl = () => page.evaluate(() => window.lab.atlas.onWebgl(window.terms["#a"].term));
await loss();
await page.waitForTimeout(200);
if (await onGl()) fail("after a lost context the pane still counts itself on WebGL");
await page.evaluate(() => document.dispatchEvent(new Event("visibilitychange")));
await page.waitForTimeout(300);
if (!(await onGl())) fail("WebGL did not come back when the page was shown again");
if (!same(before, await shot("#a"))) fail("after WebGL came back the pane draws something other than before the loss");
await loss();
await page.waitForTimeout(200);
await page.evaluate(() => document.dispatchEvent(new Event("visibilitychange")));
await page.waitForTimeout(300);
const fell = await page.evaluate(() => window.terms["#a"].fell);
if ((await onGl()) || fell.length !== 1) fail(`a second loss soon after did not keep the DOM renderer (on WebGL: ${await onGl()}, said: ${JSON.stringify(fell)})`);
const domRows = await page.evaluate(() => window.terms["#a"].term.element.querySelector(".xterm-rows")?.textContent ?? "");
if (!domRows.includes("整体界面")) fail("on the DOM renderer after the second loss, the rows do not hold the text");

await browser.close();
if (bad) process.exit(1);
console.log("atlas gate: a sibling's clear, a late face and lost contexts all leave each CJK cell as a fresh render draws it");
