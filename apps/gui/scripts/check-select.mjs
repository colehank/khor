// The selection gate (item 54): phone selection is iOS's own, over a
// transparent text layer on the grid (`src/lib/term-select.ts`), driven in
// headless Chrome against a real xterm, in the same DOM the pane builds.
//
//   a. **The layer's cells are xterm's cells.** At three font sizes, with
//      a wide (CJK) cell and a line the grid wrapped, every box of the
//      layer stands where xterm draws its cell — left, top and width
//      within half a pixel — and again after the box is resized and the
//      grid refitted.
//   b. **A copy takes the cells.** A selection over a wrapped line and the
//      next is the line joined (no break at the screen's edge), the next
//      on its own line, trailing blanks trimmed — through the `copy`
//      event the menu's 拷贝 fires, not the layer's rows.
//   c. **The focus yields and returns.** A press held 400 ms lets the
//      terminal's field go (and says so to the pane's refocus rule); a
//      selection then is reported; a tap outside it ends it and gives the
//      field back with the keyboard as it was; Esc does too, and so does
//      typing; a press on a block's prompt opens the block's menu and
//      keeps the field. Double tap takes a word, triple tap the wrapped
//      line, both by xterm's separators.
//   d. **With the keyboard up the focus never moves** (item 73, inside c):
//      a press takes the word in xterm's cells and leaves the field
//      focused, with two handles; a drag on from it extends it; the end
//      handle moves the end; the lift tells the pane the cells' text and
//      the box its menu is shown at; a tap on it shows the menu again, a
//      tap away or Esc ends it, nothing refocused, nothing sent; double
//      and triple taps the same, in the cells.
import { chromium } from "playwright-core";
import { build } from "esbuild";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const gui = fileURLToPath(new URL("..", import.meta.url));
let bad = 0;
const fail = (m) => {
  bad++;
  console.error(`select gate: ${m}`);
};

const bundle = await build({
  stdin: {
    contents: `
      import { Terminal } from "@xterm/xterm";
      import { FitAddon } from "@xterm/addon-fit";
      import * as select from "./src/lib/term-select.ts";
      window.lab = { Terminal, FitAddon, select };
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
const context = await browser.newContext({ viewport: { width: 420, height: 800 }, deviceScaleFactor: 3, hasTouch: true });
const page = await context.newPage();
page.on("pageerror", (e) => fail(`page error: ${e.message}`));
await page.setContent(`<!doctype html><meta charset=utf-8><style>${css}
  body{margin:0;background:#000;-webkit-user-select:none;user-select:none}
  #box{position:relative;width:400px;height:600px;overflow:hidden}
  #scroller{height:100%;width:100%;overflow-x:hidden;overflow-y:auto}
  #stick{position:sticky;top:0;height:100%;width:100%}
  #screen{height:100%;width:100%}
  </style><div id=box><div id=scroller><div id=stick><div id=screen></div></div><div id=room></div></div></div>`);
await page.addScriptTag({ content: bundle.outputFiles[0].text });

const TEXT =
  "plain line\r\n" +
  "中文 名.txt and more\r\n" +
  "/home/gate/a/very/long/path/that/the/grid/wraps/over/the/edge/of/the/screen/name.rs:12:3 tail\r\n" +
  "padded    \r\n" +
  "$ ";

const fresh = (fontSize) =>
  page.evaluate(
    ({ fontSize, TEXT }) =>
      new Promise((done) => {
        const { Terminal, FitAddon } = window.lab;
        window.sel?.dispose();
        window.t?.dispose();
        document.querySelector("#box").style.width = "400px";
        const term = new Terminal({ fontFamily: "Menlo, monospace", fontSize, lineHeight: 1, scrollback: 500 });
        const fit = new FitAddon();
        term.loadAddon(fit);
        term.open(document.querySelector("#screen"));
        fit.fit();
        window.t = term;
        window.fit = fit;
        term.write(TEXT, () => requestAnimationFrame(() => requestAnimationFrame(() => done({ cols: term.cols, rows: term.rows }))));
      }),
    { fontSize, TEXT },
  );

// Every box against xterm's own cell: the screen's rect over cols × rows.
const misplaced = () =>
  page.evaluate(() => {
    const term = window.t;
    const layer = window.sel.layer;
    const s = term.element.querySelector(".xterm-screen").getBoundingClientRect();
    const cw = s.width / term.cols;
    const ch = s.height / term.rows;
    const off = [];
    let boxes = 0;
    let wide = 0;
    for (const row of layer.children) {
      const y = Number(row.dataset.r) - term.buffer.active.viewportY;
      for (const box of row.children) {
        const x = Number(box.dataset.c);
        const w = term.buffer.active.getLine(Number(row.dataset.r)).getCell(x).getWidth();
        if (w === 2) wide++;
        const r = box.getBoundingClientRect();
        boxes++;
        const d = [r.left - (s.left + x * cw), r.top - (s.top + y * ch), r.width - cw * w];
        if (d.some((v) => Math.abs(v) > 0.5)) off.push(`${row.dataset.r}:${x} ${d.map((v) => v.toFixed(2)).join("/")}`);
      }
    }
    return { boxes, wide, off: off.slice(0, 5), count: off.length };
  });

const wire = () =>
  page.evaluate(() => {
    window.calls = { refocus: [], picked: [], claimed: [], at: [] };
    window.field = window.t.textarea;
    window.field.focus();
    window.up = false;
    window.sel = window.lab.select.nativeSelects({
      live: window.t,
      over: () => null,
      scroller: document.querySelector("#scroller"),
      host: document.querySelector("#stick"),
      claimable: (row) => row === window.t.buffer.active.viewportY + 6,
      claim: (row) => (window.calls.claimed.push(row), true),
      keyboardUp: () => window.up,
      keeps: () => false,
      refocus: (up, inGesture) => {
        window.calls.refocus.push([up, inGesture]);
        window.field.focus();
      },
      picked: (p, at) => {
        window.calls.picked.push(p && p.text);
        if (at) window.calls.at.push(at);
      },
    });
    // The layer is drawn at a touch's start, as on the phone.
    const scroller = document.querySelector("#scroller");
    const t = new Touch({ identifier: 9, target: scroller, clientX: 5, clientY: 5 });
    scroller.dispatchEvent(new TouchEvent("touchstart", { touches: [t], changedTouches: [t], bubbles: true }));
    scroller.dispatchEvent(new TouchEvent("touchend", { touches: [], changedTouches: [t], bubbles: true }));
  });

// ── a ──
for (const size of [11, 14, 18]) {
  const geo = await fresh(size);
  await wire();
  const m = await misplaced();
  console.log(`select gate: a · font ${size}: ${geo.cols}×${geo.rows}, ${m.boxes} boxes (${m.wide} wide), ${m.count} off`);
  if (m.boxes < 40 || m.wide < 2) fail(`probe dead: a · font ${size}: the layer has ${m.boxes} boxes, ${m.wide} wide`);
  if (m.count) fail(`a · font ${size}: ${m.count} boxes stand off their cells (row:col dx/dy/dw): ${m.off.join(", ")}`);
}
// After a resize: narrower, refitted, the layer drawn at the next touch.
await page.evaluate(() => {
  document.querySelector("#box").style.width = "290px";
  window.fit.fit();
});
await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
await page.touchscreen.tap(30, 30);
await page.waitForTimeout(100);
{
  const m = await misplaced();
  const cols = await page.evaluate(() => window.t.cols);
  console.log(`select gate: a · after a resize to ${cols} columns: ${m.boxes} boxes, ${m.count} off`);
  if (m.count) fail(`a · after a resize: ${m.count} boxes stand off their cells: ${m.off.join(", ")}`);
}

// ── b ──
await fresh(14);
await wire();
{
  const got = await page.evaluate(() => {
    const b = window.t.buffer.active;
    let wrapped = -1;
    for (let y = 0; y < window.t.rows; y++) if (b.getLine(y)?.isWrapped) { wrapped = y; break; }
    // From the path's first cell to the end of 「padded」's row.
    const ok = window.lab.select.selectCells(window.sel.layer, { row: wrapped - 1, col: 0 }, { row: wrapped + 2, col: window.t.cols - 1 });
    const data = new DataTransfer();
    const ev = new ClipboardEvent("copy", { clipboardData: data, bubbles: true, cancelable: true });
    document.dispatchEvent(ev);
    return { ok, wrapped, copied: data.getData("text/plain"), cancelled: ev.defaultPrevented };
  });
  const path = "/home/gate/a/very/long/path/that/the/grid/wraps/over/the/edge/of/the/screen/name.rs:12:3 tail";
  // The path's rows may wrap more than once: the copy is the line whole.
  const want = `${path}\npadded`;
  console.log(`select gate: b · copy over the wrapped line: ${JSON.stringify(got.copied)}`);
  if (got.wrapped < 1 || !got.ok) fail(`probe dead: b · no wrapped row to select over (${got.wrapped})`);
  else if (!got.cancelled || !got.copied.startsWith(want)) fail(`b · the copy took ${JSON.stringify(got.copied)}, not ${JSON.stringify(want)} — the cells, wrapped rows joined, blanks trimmed`);
}

// ── c ──
const state = () =>
  page.evaluate(() => ({
    focused: document.activeElement === window.field,
    holds: window.lab.select.selectionHolds(),
    selected: String(window.getSelection()),
    refocus: window.calls.refocus.slice(),
    picked: window.calls.picked.slice(),
    claimed: window.calls.claimed.slice(),
    sent: window.sent.join(""),
  }));
const cellPoint = (row, col) =>
  page.evaluate(({ row, col }) => {
    const s = window.t.element.querySelector(".xterm-screen").getBoundingClientRect();
    return { x: s.left + (col + 0.5) * (s.width / window.t.cols), y: s.top + (row + 0.5) * (s.height / window.t.rows) };
  }, { row, col });
// A touch's moves and lift go to where it began, as on a phone.
const touch = (type, p) =>
  page.evaluate(({ type, p }) => {
    if (type === "touchstart") window.touchAt = document.elementFromPoint(p.x, p.y);
    const target = window.touchAt;
    const t = new Touch({ identifier: 1, target, clientX: p.x, clientY: p.y });
    target.dispatchEvent(new TouchEvent(type, { touches: type === "touchend" ? [] : [t], changedTouches: [t], bubbles: true, cancelable: true }));
  }, { type, p });
const keyOnPage = (key, keyCode) =>
  page.evaluate(({ key, keyCode }) => {
    const ev = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
    Object.defineProperty(ev, "keyCode", { get: () => keyCode });
    (document.activeElement ?? document.body).dispatchEvent(ev);
  }, { key, keyCode });

await fresh(14);
await page.evaluate(() => {
  window.sent = [];
  window.t.onData((d) => window.sent.push(d));
  // The keyboard down: the field holds the focus with `inputmode=none`.
  window.t.textarea.setAttribute("inputmode", "none");
});
await wire();
{
  // At rest, keyboard down: the field is let go (WebKit decides at a
  // touch's start whether a long press selects or works in the field).
  await page.waitForTimeout(600);
  const rest = await state();
  console.log(`select gate: c · keyboard down, at rest: field ${rest.focused ? "held" : "let go"}`);
  if (rest.focused || !rest.holds) fail(`c · with the keyboard down the field still holds the focus at rest (${JSON.stringify(rest)}) — iOS's selection cannot start in a focused field`);
  // A hardware keyboard still types at once: a character, then Enter.
  await keyOnPage("a", 65);
  await page.waitForTimeout(50);
  await keyOnPage("Enter", 13);
  await page.waitForTimeout(50);
  const typed = await state();
  console.log(`select gate: c · keys at rest reached the terminal as ${JSON.stringify(typed.sent)}`);
  if (typed.sent !== "a\r") fail(`c · keys typed with the terminal at rest sent ${JSON.stringify(typed.sent)}, not "a\\r" — a hardware keyboard types at once (item 11)`);
  // A long press with the keyboard down is iOS's: here, its selection.
  await page.evaluate(() => window.lab.select.selectCells(window.sel.layer, { row: 0, col: 0 }, { row: 0, col: 4 }));
  await page.waitForTimeout(50);
  const up = await state();
  if (up.selected !== "plain" || up.picked.at(-1) !== "plain" || !up.holds) fail(`c · a selection with the keyboard down is not reported (${JSON.stringify(up)})`);
  // Esc ends it and sends nothing; typing ends it and is sent.
  await keyOnPage("Escape", 27);
  await page.waitForTimeout(50);
  const esc = await state();
  if (esc.selected !== "" || esc.sent !== "a\r" || esc.picked.at(-1) !== null) fail(`c · Esc over a selection did not end it, or was sent (${JSON.stringify(esc)})`);
  await page.evaluate(() => window.lab.select.selectCells(window.sel.layer, { row: 0, col: 0 }, { row: 0, col: 4 }));
  await page.waitForTimeout(50);
  await keyOnPage("b", 66);
  await page.waitForTimeout(50);
  const b = await state();
  if (b.selected !== "" || b.sent !== "a\rb") fail(`c · typing over a selection did not end it and reach the terminal (${JSON.stringify(b)})`);
}
{
  // **Keyboard up: khor's own selection, the focus never moves** (item 73).
  // A press held 400 ms takes the word in xterm's cells and keeps the
  // field; a drag on from it extends it; the handles adjust it; the lift
  // tells the pane with the box the menu is shown at; the text is the
  // cells'. A tap on it shows the menu again, a tap away ends it, and
  // neither touches the focus.
  await page.evaluate(() => {
    window.getSelection().removeAllRanges();
    window.field.setAttribute("inputmode", "text");
    window.field.focus();
    window.up = true;
    window.calls.refocus.length = 0;
  });
  await page.waitForTimeout(600);
  const held0 = await state();
  if (!held0.focused) fail(`c · with the keyboard up the field did not keep the focus (${JSON.stringify(held0)})`);
  const own = () =>
    page.evaluate(() => ({
      focused: document.activeElement === window.field,
      cells: window.t.getSelection(),
      dom: String(window.getSelection()),
      handles: [...document.querySelectorAll("[data-term-handle]")].filter((h) => h.style.display !== "none").length,
      up: window.lab.select.selectionUp(),
      at: window.calls.at.at(-1) ?? null,
      picked: window.calls.picked.at(-1),
      refocus: window.calls.refocus.slice(),
    }));
  const p = await cellPoint(0, 2);
  await touch("touchstart", p);
  await page.waitForTimeout(450);
  const pressed = await own();
  await touch("touchend", p);
  await page.waitForTimeout(50);
  const lifted = await own();
  console.log(`select gate: c · keyboard up: press → field ${pressed.focused ? "kept" : "let go"}, cells ${JSON.stringify(pressed.cells)}, ${pressed.handles} handles; lift → told ${JSON.stringify(lifted.picked)} at ${lifted.at ? "a box" : "nothing"}`);
  if (!pressed.focused || pressed.cells !== "plain" || pressed.dom !== "" || pressed.handles !== 2 || !pressed.up) {
    fail(`c · keyboard up, a press held 400 ms did not take the word in the cells with the field kept and two handles (${JSON.stringify(pressed)}) — the keyboard would fall`);
  }
  if (lifted.picked !== "plain" || !lifted.at || lifted.at.w <= 0 || lifted.at.h <= 0) fail(`c · keyboard up, the lift did not tell the pane the word and the menu's box (${JSON.stringify(lifted)})`);
  // A tap on it: the menu again; a tap away: gone, the focus as it was.
  const n = await page.evaluate(() => window.calls.at.length);
  const onIt = await page.evaluate(({ x, y }) => window.sel.tapEnds(x, y), p);
  const again = await page.evaluate(() => window.calls.at.length);
  await page.waitForTimeout(400);
  const away = await cellPoint(3, 1);
  const took = await page.evaluate(({ x, y }) => window.sel.tapEnds(x, y), away);
  const gone = await own();
  console.log(`select gate: c · keyboard up: tap on it → ${again > n ? "menu" : "nothing"}; tap away → ${took ? "taken" : "not taken"}, cells ${JSON.stringify(gone.cells)}, field ${gone.focused ? "kept" : "let go"}`);
  if (!onIt || again <= n) fail(`c · keyboard up, a tap on the selection did not show its menu again`);
  if (!took || gone.cells !== "" || gone.handles !== 0 || gone.up || !gone.focused || gone.refocus.length || gone.picked !== null) {
    fail(`c · keyboard up, a tap away did not end it with the focus untouched (${JSON.stringify(gone)})`);
  }
  // A drag on from the press extends it, over the wrapped line, and the
  // text told is the cells' (wrapped rows joined, blanks trimmed).
  const rows = await page.evaluate(() => {
    const b = window.t.buffer.active;
    let wrapped = -1;
    let padded = -1;
    for (let y = 0; y < window.t.rows; y++) {
      if (wrapped < 0 && b.getLine(y + 1)?.isWrapped) wrapped = y;
      if (b.getLine(y)?.translateToString(true).trimEnd() === "padded") padded = y;
    }
    return { wrapped, padded };
  });
  const path = "/home/gate/a/very/long/path/that/the/grid/wraps/over/the/edge/of/the/screen/name.rs:12:3";
  {
    const a = await cellPoint(rows.wrapped, 3);
    const b = await cellPoint(rows.padded, 5);
    await touch("touchstart", a);
    await page.waitForTimeout(450);
    const word = await own();
    for (let i = 1; i <= 4; i++) await touch("touchmove", { x: a.x + ((b.x - a.x) * i) / 4, y: a.y + ((b.y - a.y) * i) / 4 });
    await touch("touchend", b);
    await page.waitForTimeout(50);
    const out = await own();
    console.log(`select gate: c · keyboard up: press on the wrapped path, drag to 「padded」 → told ${JSON.stringify(out.picked)}`);
    if (word.cells.replace(/\n/g, "") !== path) fail(`c · keyboard up, the press took ${JSON.stringify(word.cells)}, not the wrapped word`);
    if (out.picked !== `${path} tail\npadded` || !out.focused) fail(`c · keyboard up, a drag on from the press told ${JSON.stringify(out.picked)}, not the cells' text ${JSON.stringify(`${path} tail\npadded`)}`);
  }
  // The end handle, dragged back to 「pad」.
  {
    const h = await page.evaluate(() => {
      const r = document.querySelector('[data-term-handle="end"]').getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.bottom - 10 };
    });
    const to = await cellPoint(rows.padded, 2);
    const dy = h.y - (await cellPoint(rows.padded, 5)).y;
    await touch("touchstart", h);
    await touch("touchmove", { x: to.x, y: to.y + dy });
    await touch("touchend", { x: to.x, y: to.y + dy });
    await page.waitForTimeout(50);
    const out = await own();
    console.log(`select gate: c · keyboard up: the end handle to 「pad」 → told ${JSON.stringify(out.picked)}`);
    if (out.picked !== `${path} tail\npad` || !out.focused) fail(`c · keyboard up, the end handle's drag told ${JSON.stringify(out.picked)}, not ${JSON.stringify(`${path} tail\npad`)}`);
  }
  // Esc ends it and is not sent; the field keeps the focus.
  {
    const sent0 = (await state()).sent;
    await keyOnPage("Escape", 27);
    await page.waitForTimeout(50);
    const out = await own();
    const sent = (await state()).sent;
    if (out.cells !== "" || out.handles || sent !== sent0 || !out.focused) fail(`c · keyboard up, Esc did not end khor's selection, or was sent (${JSON.stringify(out)}, sent ${JSON.stringify(sent.slice(sent0.length))})`);
  }
  // Double tap a word, triple tap the line, the focus kept.
  {
    const q = await cellPoint(rows.wrapped, 3);
    await page.waitForTimeout(400);
    let r;
    for (let i = 0; i < 2; i++) r = await page.evaluate(({ x, y }) => window.sel.tapEnds(x, y), q);
    const two = await own();
    r = await page.evaluate(({ x, y }) => window.sel.tapEnds(x, y), q);
    const three = await own();
    console.log(`select gate: c · keyboard up: double tap → ${JSON.stringify(two.picked)}; triple tap → ${JSON.stringify(three.picked)}`);
    if (two.picked !== path || !two.focused || two.dom !== "") fail(`c · keyboard up, a double tap told ${JSON.stringify(two.picked)} (field ${two.focused}), not the word in the cells`);
    if (!r || three.picked !== `${path} tail` || !three.focused) fail(`c · keyboard up, a triple tap told ${JSON.stringify(three.picked)}, not the line`);
    await page.evaluate(() => window.sel.end());
  }
  // Under the key panel (no keyboard, the field focused): item 54's road.
  await page.evaluate(() => {
    window.up = false;
    window.field.setAttribute("inputmode", "none");
  });
}
{
  // Double tap: a word by xterm's separators; triple tap: the wrapped line.
  const row = await page.evaluate(() => {
    const b = window.t.buffer.active;
    for (let y = 0; y < window.t.rows; y++) if (b.getLine(y + 1)?.isWrapped) return y;
    return -1;
  });
  const p = await cellPoint(row, 3);
  const taps = async (n) => {
    let r;
    for (let i = 0; i < n; i++) r = await page.evaluate(({ x, y }) => window.sel.tapEnds(x, y), p);
    await page.waitForTimeout(50);
    return { took: r, text: (await state()).picked.at(-1) };
  };
  await page.waitForTimeout(400);
  const two = await taps(2);
  await page.evaluate(() => window.getSelection().removeAllRanges());
  await page.waitForTimeout(400);
  const three = await taps(3);
  console.log(`select gate: c · double tap → ${JSON.stringify(two.text)}; triple tap → ${JSON.stringify(three.text)}`);
  const path = "/home/gate/a/very/long/path/that/the/grid/wraps/over/the/edge/of/the/screen/name.rs:12:3";
  if (!two.took || two.text !== path) fail(`c · a double tap took ${JSON.stringify(two.text)}, not the wrapped word ${JSON.stringify(path)}`);
  if (!three.took || three.text !== `${path} tail`) fail(`c · a triple tap took ${JSON.stringify(three.text)}, not the whole wrapped line`);
  await page.evaluate(() => window.getSelection().removeAllRanges());
  await page.waitForTimeout(50);
}
{
  // A press on a block's prompt: the block's menu, and the row is not
  // selectable.
  await page.evaluate(() => window.field.focus());
  await page.waitForTimeout(400);
  const vy = await page.evaluate(() => window.t.buffer.active.viewportY);
  const p = await cellPoint(6, 1);
  await touch("touchstart", p);
  await page.waitForTimeout(450);
  await touch("touchend", p);
  const s = await state();
  const none = await page.evaluate((r) => getComputedStyle(window.sel.layer.querySelector(`[data-r="${r}"]`)).userSelect, vy + 6);
  if (s.claimed.at(-1) !== vy + 6 || none !== "none" || s.selected !== "") fail(`c · a press on a block's prompt did not open its menu, or selected (${JSON.stringify(s)}, user-select ${none})`);
}

await browser.close();
if (bad) {
  console.error(`select gate: ${bad} failure(s)`);
  process.exit(1);
}
console.log("select gate: the layer stands on xterm's cells, a copy takes the cells, the focus yields to a selection and comes back");
