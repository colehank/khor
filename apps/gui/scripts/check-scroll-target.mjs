// Real xterm input routing: an inner pane owns mouse reports only when it
// requested them. Alternate screen alone gets standard paging keys.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";
import { chromium } from "playwright-core";
const gui = fileURLToPath(new URL("..", import.meta.url));
const bundle = await build({ stdin: { contents: `
  import { Terminal } from '@xterm/xterm';
  import { scrollTarget, pageScroll, wheelsToPages } from './src/lib/term-scroll-target';
  import { scrollsNatively, SETTLE_MS } from './src/lib/term-scroller';
  import { wheelsToShellTmux } from './src/lib/term-shell-tmux';
  window.lab = { Terminal, scrollTarget, pageScroll, wheelsToPages, scrollsNatively, SETTLE_MS, wheelsToShellTmux };
`, resolveDir: gui, loader: "ts" }, bundle: true, format: "iife", write: false });
const browser = await chromium.launch({ channel: "chrome", headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 600, height: 600 } });
  await page.setContent(`<style>${readFileSync(`${gui}/node_modules/@xterm/xterm/css/xterm.css`, "utf8")}
    #box { width:500px; height:400px; overflow:hidden; }
    #scroller { height:100%; overflow:auto; } #screen { position:sticky; top:0; }
    </style><div id=box><div id=scroller><div id=screen></div><div id=room></div></div></div>`);
  await page.addScriptTag({ content: bundle.outputFiles[0].text });
  const result = await page.evaluate(async () => {
    const { Terminal, scrollTarget, pageScroll, wheelsToPages, scrollsNatively, SETTLE_MS, wheelsToShellTmux } = window.lab;
    const term = new Terminal({ cols: 60, rows: 24, fontSize: 14 });
    const box = document.querySelector('#box'), scroller = document.querySelector('#scroller');
    const screen = document.querySelector('#screen');
    term.open(screen);
    const write = bytes => new Promise(resolve => term.write(bytes, resolve));
    const frames = () => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    const data = []; term.onData(bytes => data.push(bytes));
    let pane = null, downstream = 0;
    const unpage = wheelsToPages(box, term, () => pane);
    // Same-node listeners must not process an intercepted page twice.
    const unshell = wheelsToShellTmux(box, term, { front: () => true, lost: () => {} }, async () => { downstream++; return true; });
    const wheel = rows => term.element.querySelector('.xterm-screen').dispatchEvent(new WheelEvent('wheel', {
      bubbles: true, cancelable: true, deltaY: rows, deltaMode: WheelEvent.DOM_DELTA_LINE,
      clientX: 30, clientY: 30,
    }));
    const targets = {};
    targets.normal = scrollTarget(term);
    await write('\x1b[?1049h\x1b[?1000h\x1b[?1006h');
    pane = { alternate: true, mouse: false };
    targets.innerPages = scrollTarget(term, pane);
    wheel(-1); wheel(-22); wheel(-1);
    const pages = data.join(''); const duplicates = downstream;
    data.length = 0;
    pane = { alternate: true, mouse: true };
    targets.innerMouse = scrollTarget(term, pane);
    wheel(-1);
    const mouse = data.join('');
    pane = { alternate: false, mouse: false };
    targets.innerHistory = scrollTarget(term, pane);
    await write('\x1b[?1000l\x1b[?1006l');
    targets.directPages = scrollTarget(term);
    data.length = 0;
    pane = { alternate: true, mouse: false };
    // The same native travel used on a phone produces pages, not discarded
    // mouse reports. Geometry and resting positions produce no key input.
    const native = scrollsNatively(term, scroller, document.querySelector('#room'), screen, { wheel });
    await frames();
    const center = scroller.scrollTop;
    const row = term.element.querySelector('.xterm-screen').getBoundingClientRect().height / term.rows;
    scroller.dispatchEvent(new Event('touchstart'));
    scroller.scrollTop = center - row * 24;
    await frames();
    const nativeKeys = data.join('');
    const nativeDuplicates = downstream;
    scroller.dispatchEvent(new Event('touchend'));
    await new Promise(resolve => setTimeout(resolve, SETTLE_MS + 30));
    const afterSettle = data.join('');
    native.stop(); unpage(); unshell(); term.dispose();
    // A deterministic gesture clock covers accumulation and reversal
    // independently of browser frame timing.
    let clock = 0; const batches = [];
    const pageRows = pageScroll(bytes => batches.push(bytes), () => 24, () => clock);
    pageRows(-1); const first = batches.join('');
    for (let i = 0; i < 22; i++) { clock += 1; pageRows(-1); }
    const carried = batches.join('');
    pageRows(-1); const full = batches.join('');
    pageRows(1); const reversed = batches.join('');
    clock += SETTLE_MS + 1; pageRows(1); const newGesture = batches.join('');
    pageRows(46); const large = batches.join('');
    pageRows(0); pageRows(NaN); pageRows(Infinity);
    return { targets, pages, duplicates, nativeDuplicates, mouse, nativeKeys, afterSettle,
      first, carried, full, reversed, newGesture, large, final: batches.join('') };
  });
  assert.deepEqual(result.targets, { normal: 'history', innerPages: 'pages', innerMouse: 'mouse', innerHistory: 'history', directPages: 'pages' });
  assert.equal(result.pages, '\x1b[5~\x1b[5~');
  assert.equal(result.duplicates, 0);
  assert.equal(result.nativeDuplicates, 0);
  assert.match(result.mouse, /^\x1b\[<64;/);
  assert.match(result.nativeKeys, /^(\x1b\[5~)+$/);
  assert.equal(result.afterSettle, result.nativeKeys);
  assert.equal(result.first, '\x1b[5~');
  assert.equal(result.carried, result.first);
  assert.equal(result.full, '\x1b[5~\x1b[5~');
  assert.equal(result.reversed, result.full + '\x1b[6~');
  assert.equal(result.newGesture, result.reversed + '\x1b[6~');
  assert.equal(result.large, result.newGesture + '\x1b[6~\x1b[6~');
  assert.equal(result.final, result.large);
  console.log('scroll target gate: inner mouse capability, page accumulation/reversal/rest, native touch travel and no duplicate input passed');
} finally {
  await browser.close();
}
