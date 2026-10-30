// Actual production ChatView/worker/reader/hook. Only external API/clipboard IO is stubbed.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync, mkdirSync, readFileSync, writeFileSync, readdirSync, rmSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {createServer} from 'vite';
import {chromium} from 'playwright-core';

const gui = fileURLToPath(new URL('..', import.meta.url));
const evidence = process.env.HISTORY_SEARCH_UI_EVIDENCE;
assert(evidence, 'Provide a private evidence directory'); mkdirSync(evidence, {recursive: true});
const fixture = mkdtempSync(join(gui, '.history-search-check-'));
const files = directory => readdirSync(directory, {withFileTypes: true}).flatMap(item => item.isDirectory() ? files(join(directory, item.name)) : [join(directory, item.name)]);
const digest = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const fingerprint = () => Object.fromEntries([...files(join(gui, 'src')), fileURLToPath(import.meta.url)].map(path => [path.slice(gui.length + 1), digest(path)]));
const before = fingerprint(), results = [], errors = [];
writeFileSync(join(evidence, 'source-before.json'), JSON.stringify(before, null, 2));
writeFileSync(join(fixture, 'index.html'), '<div id="root"></div><script type="module" src="./probe.tsx"></script>');
writeFileSync(join(fixture, 'probe.tsx'), `
import React,{useState} from 'react'; import {createRoot} from 'react-dom/client';
import {ChatView} from '/src/views/ChatView'; import {TooltipProvider} from '/src/components/ui/tooltip';
import {gui} from '/src/gen/catalog'; import '/src/styles/tokens.css'; import '/src/styles/app.css';
function Probe(){const [binding,change]=useState('bind-A'),[shown,show]=useState(true),[hidden,hide]=useState(false);
window.__probe={change,show,hide,words:()=>gui};return <TooltipProvider><div style={{height:hidden?'0px':'100vh',overflow:'hidden',display:'flex',flexDirection:'column',minHeight:0}}>{shown&&<ChatView id="observe/fixture" still binding={binding}/>}</div></TooltipProvider>;}
createRoot(document.getElementById('root')).render(<Probe/>);
`);

function transport() {
  const encoder = new TextEncoder();
  const bytes = encoder.encode(JSON.stringify({kind: 'prompt', turn: 'fixture', text: 'x'.repeat(70000) + 'giantNeedle中文🙂' + 'z'.repeat(50000), images: [], files: []}) + '\n');
  const broken = encoder.encode('{invalid:' + 'q'.repeat(70000) + '\n');
  const ref = (bad = false, tag = 'A') => ({start: bad ? 1_281_000_000 : 1_280_000_000, end: (bad ? 1_281_000_000 : 1_280_000_000) + (bad ? broken.length : bytes.length), cursor: `${bad ? 'raw-bad' : 'raw-good'}:${tag}`, missing_start: false, missing_end: false, item: null});
  const wire = window.__historyWire = {calls: [], unknown: [], leases: {}, jobs: {}, held: null, heldKey: null, mode: null, trace: [], holdCopy: false, holdInitial: false,
    clipboard: {text: '', writes: 0, rejected: 0, activation: false}, order: [],
    release: () => { const held = wire.held; wire.held = null; wire.heldKey = null; held?.(); },
    hidden: value => {Object.defineProperty(document, 'hidden', {configurable: true, value});document.dispatchEvent(new Event('visibilitychange'));},
  };
  const geometry = node => {
    if (!node) return null;
    const rect = node.getBoundingClientRect(), style = getComputedStyle(node);
    return {x: rect.x, y: rect.y, width: rect.width, height: rect.height, clientHeight: node.clientHeight,
      paddingTop: style.paddingTop, paddingBottom: style.paddingBottom, display: style.display};
  };
  const trace = (event, extra = {}) => {
    if (!wire.mode) return;
    const scroll = document.querySelector('[data-chat-scroll]'), root = scroll?.parentElement?.parentElement;
    wire.trace.push({event, mode: wire.mode, at: performance.now(), sequence: wire.calls.length,
      hidden: document.hidden, heldKey: wire.heldKey, searchCount: document.querySelectorAll('[data-history-search]').length,
      parent: geometry(root?.parentElement), root: geometry(root), scroll: geometry(scroll),
      leases: Object.fromEntries(Object.entries(wire.leases).map(([key, lease]) => [key, {...lease}])), ...extra});
    if (wire.trace.length > 128) wire.trace.shift();
  };
  wire.capture = trace;
  // Observe the existing callbacks, preserving their entries/receiver and
  // original geometry semantics. No additional ResizeObserver is created.
  const NativeResizeObserver = window.ResizeObserver;
  window.ResizeObserver = class extends NativeResizeObserver {
    constructor(callback) {
      super((entries, observer) => {
        trace('resize-before', {content: entries.map(entry => ({width: entry.contentRect.width, height: entry.contentRect.height, scroller: entry.target.hasAttribute('data-chat-scroll')}))});
        callback.call(observer, entries, observer);
        queueMicrotask(() => trace('resize-after'));
      });
    }
  };
  const note = (key, text, record = null, starts_message = true) => ({key, record, starts_message, frame: {kind: 'note', update: {update: {sessionUpdate: 'agent_message_chunk', content: {type: 'text', text}}}}});
  let serial = 0;
  const invoke = async (cmd, args = {}) => {
    wire.calls.push({cmd, args});
    trace(cmd, {requestKey: args.requestKey});
    if (cmd === 'plugin:event|listen') return 0;
    if (cmd === 'plugin:event|unlisten') return null;
    if (cmd === 'devices' || cmd === 'workspaces') return [];
    if (cmd === 'history_begin') { const key = `lease-${++serial}`; wire.leases[key] = {binding: args.binding, used: false, claimed: false, cancelled: false}; return key; }
    if (cmd === 'history_cancel') {
      const lease = wire.leases[args.requestKey];
      if (lease) {lease.cancelled = true;if (!lease.claimed) delete wire.leases[args.requestKey];}
      return null;
    }
    if (['history_page', 'history_record', 'history_projection'].includes(cmd)) {
      const lease = wire.leases[args.requestKey];
      if (!lease || lease.used || lease.cancelled || lease.binding !== args.binding) throw new Error('invalid or reused fixture read lease');
      lease.used = true;
      // Pure projection uses the client lease locally; its home request stays
      // pending until explicit Cancel. Page/record claim the actual IO owner.
      lease.claimed = cmd !== 'history_projection';
    }
    if (cmd === 'history_page') {
      const tag = args.binding.slice(-1);
      if(args.cursor !== null && (!new RegExp(`^f:${tag}:[0-9]+$`).test(args.cursor))) throw new Error('page cursor belongs to another fixed source');
      const at = args.cursor ? Number(args.cursor.split(':').at(-1)) : 1300;
      const start = args.newer ? at : Math.max(0, at - 128), end = args.newer ? Math.min(1300, at + 128) : at;
      const references = end > 1281 && start <= 1280 ? [ref(false, tag), ref(true, tag)] : [];
      // Fixed complete Native messages, not SDK chunks. These DTO ranges and
      // cursors are scene facts; no product normalization/source algorithm runs here.
      const entries = Array.from({length: end - start}, (_, k) => start + k)
        .filter(i => i !== 1280 && i !== 1281).map(i => note(`${i * 1_000_000}:0`, `${tag} row ${i}${i === 50 ? ' needleOld' : ''}`,
          {start:i * 1_000_000,end:(i + 1) * 1_000_000,item:0,cursor:`f:${tag}:${i}`,missing_start:false,missing_end:false}));
      const page = {entries, references, older: start ? `f:${tag}:${start}` : null, newer: `f:${tag}:${end}`, has_more: args.newer && end < 1300, reset: false, encoded_bytes: 0};
      page.encoded_bytes = encoder.encode(JSON.stringify(page)).length + 20;
      if (!args.newer && ((wire.holdCopy && args.newer) || (wire.holdInitial && args.cursor === null))) {
        wire.holdCopy = false; wire.holdInitial = false; wire.order.push('page-held'); wire.heldKey = args.requestKey; return new Promise(resolve => {wire.held = () => resolve(page);}).finally(() => {delete wire.leases[args.requestKey];});
      }
      // Node's claimed Request drops on completed page/record IO. Only
      // held IO and pure projection's pending home lease need cancel.
      delete wire.leases[args.requestKey];
      return page;
    }
    if (cmd === 'history_record') {
      const bad = args.cursor.startsWith('raw-bad:'), selected = ref(bad, args.binding.slice(-1)), all = bad ? broken : bytes;
      if (args.cursor !== selected.cursor) throw new Error('raw cursor belongs to another binding');
      const part = all.subarray(args.from - selected.start, args.from - selected.start + 65536);
      let value = '';for (const byte of part) value += String.fromCharCode(byte);
      const end = args.from + part.length;
      delete wire.leases[args.requestKey];
      return {offset: args.from, data: btoa(value), next: end < selected.end ? end : null};
    }
    if (cmd === 'history_projection') {
      const record = {...ref(args.cursor.startsWith('raw-bad:'), args.binding.slice(-1)), start: args.start, end: args.end, missing_start: true, missing_end: true};
      const value = args.scaffold ? JSON.parse(args.scaffold) : null;
      return {record, transforms: [], complete: !!value, entries: value ? [note('projection:0', value.text)] : []};
    }
    if (cmd === 'history_export_start') {
      const selected = ref(args.cursor.startsWith('raw-bad:'), args.binding.slice(-1));
      if (args.cursor !== selected.cursor) throw new Error('export cursor belongs to another binding');
      if (args.start !== selected.start || args.end !== selected.end) throw new Error('export range differs from selected original');
      wire.jobs[args.transfer] = {binding: args.binding, start: args.start, end: args.end, cancelled: false, saved: false};
      return {moved: args.end - args.start, total: args.end - args.start, ready: true, cancelled: false, error: null};
    }
    if (cmd === 'file_handoff_save') { const job = wire.jobs[args.id]; if (!job || job.cancelled) throw new Error('missing job'); job.saved = true; return true; }
    if (cmd === 'file_handoff_cancel') {if (wire.jobs[args.id]) wire.jobs[args.id].cancelled = true;return null;}
    if (cmd === 'file_handoff_state') {const job = wire.jobs[args.id];return {moved: job.end - job.start, total: job.end - job.start, ready: true, cancelled: job.cancelled, error: null};}
    wire.unknown.push(cmd); throw new Error(`unknown fixture API: ${cmd}`);
  };
  window.__TAURI_INTERNALS__ = {metadata: {currentWindow: {label: 'main'}, currentWebview: {label: 'main', windowLabel: 'main'}}, transformCallback: () => 0, unregisterCallback: () => {}, convertFileSrc: path => path, invoke};
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {unregisterListener: () => {}};
  Object.defineProperty(navigator, 'clipboard', {configurable: true, value: {
    write: async items => {wire.clipboard.writes++;wire.clipboard.activation = navigator.userActivation.isActive;wire.order.push('clipboard-write');try {wire.clipboard.text = await (await items[0].getType('text/plain')).text();}catch(error){wire.clipboard.rejected++;throw error;}},
    writeText: async text => {wire.clipboard.text = text;},
  }});
}

let server, browser, page;
const passed = name => {results.push(name);console.log('PASS', name);};
const until = async (check, label) => {const deadline = Date.now() + 15000;while (!await check()) {assert(Date.now() < deadline, label);await new Promise(resolve => setTimeout(resolve, 30));}};
try {
  server = await createServer({configFile: join(gui, 'vite.config.ts'), root: gui, cacheDir: join(fixture, 'cache'), optimizeDeps: {entries: [join(fixture, 'index.html')]}, server: {host: '127.0.0.1', port: 23841, strictPort: true}, clearScreen: false}); await server.listen();
  browser = await chromium.launch({channel: 'chrome', headless: true});
  page = await browser.newPage({viewport: {width: 1100, height: 800}});page.setDefaultTimeout(15000);
  page.on('pageerror', error => errors.push(String(error)));
  await page.addInitScript(transport);
  await page.goto(`http://127.0.0.1:23841/${fixture.split('/').at(-1)}/index.html`);
  await page.locator('[data-history-find-open]').click();
  const words = await page.evaluate(() => window.__probe.words());
  const field = page.locator('[data-history-search] input');
  const submit = () => page.getByRole('button', {name: words.search_open, exact: true}).click();
  await field.fill('needleOld');await submit();
  await page.locator('[data-history-find-hit]').filter({hasText: 'A row 50'}).waitFor();
  const calls = await page.evaluate(() => window.__historyWire.calls.filter(call => call.cmd === 'history_page'));
  assert(calls.length > 8, 'search reaches beyond the retained reader cache');
  await page.locator('[data-history-find-hit]').first().click();
  await page.locator('[data-history-key="50000000:0:0"]').waitFor();
  await until(() => page.evaluate(() => {
    const row = document.querySelector('[data-history-key="50000000:0:0"]').getBoundingClientRect();
    const view = document.querySelector('[data-chat-scroll]').getBoundingClientRect();
    return row.top >= view.top && row.bottom <= view.bottom;
  }), 'selected source result is inside the actual scroll viewport');
  assert(await page.locator('[data-history-key]').count() <= 512);
  passed('source search beyond retained DOM and exact result navigation');

  await page.locator('[data-history-find-hit]').first().click({button: 'right'});
  await page.getByRole('menuitem', {name: words.history_copy, exact: true}).click();
  await until(() => page.evaluate(() => window.__historyWire.clipboard.text === 'A row 50 needleOld'), 'complete source-backed copy');
  assert.equal(await page.evaluate(() => window.__historyWire.clipboard.activation), true);
  passed('source-backed copy submits promised clipboard data during the click');

  await field.fill('giantNeedle');await submit();await page.locator('[data-history-find-hit]').first().waitFor();
  await page.locator('[data-history-find-hit]').first().click();
  await page.locator('[data-history-find-context] mark').filter({hasText: 'giantNeedle'}).waitFor();
  assert((await page.locator('[data-history-find-context] pre').textContent()).length < 1000);
  await page.locator('[data-history-find-context]').getByRole('button', {name: words.history_copy, exact: true}).click();
  await page.getByText(words.history_copy_large, {exact: true}).waitFor();
  assert.equal(await page.evaluate(() => window.__historyWire.clipboard.text), 'A row 50 needleOld');
  passed('giant context stays bounded and full-part copy refuses truncation');

  await page.locator('[data-chat-bottom]').click();
  await until(() => page.locator('[data-history-records] button').count().then(count => count >= 2), 'latest raw references');
  await page.locator('[data-history-records] button').nth(1).click({button: 'right'});
  await page.getByRole('menuitem', {name: words.history_export, exact: true}).click();
  await until(() => page.evaluate(() => Object.values(window.__historyWire.jobs).some(job => job.start === 1_281_000_000 && job.saved)), 'malformed raw record export without a search hit');
  await page.getByText(words.history_exported, {exact: true}).waitFor();
  passed('right-click raw export reaches the native original handoff without searchable JSON');

  await field.fill('needleOld');
  assert.equal(await page.getByText(words.history_exported, {exact: true}).count(), 0, 'a new local intent clears an old idle export result');
  await submit();await page.locator('[data-history-find-hit]').first().waitFor();
  await page.evaluate(() => {window.__historyWire.clipboard.text = 'OWNER_BARRIER';window.__historyWire.holdCopy = true;});
  await page.locator('[data-history-find-hit]').first().click({button: 'right'});
  await page.getByRole('menuitem', {name: words.history_copy, exact: true}).click();
  await until(() => page.evaluate(() => !!window.__historyWire.held), 'copy action held on the original source');
  assert(await page.evaluate(() => window.__historyWire.order.lastIndexOf('clipboard-write') < window.__historyWire.order.lastIndexOf('page-held')), 'clipboard operation is submitted before its body IO completes');
  await page.evaluate(() => window.__probe.change('bind-B'));
  await until(() => page.evaluate(() => window.__historyWire.calls.some(call => call.cmd === 'history_page' && call.args.binding === 'bind-B')), 'new source reader');
  await page.evaluate(() => window.__historyWire.release());
  if (!await page.locator('[data-history-search]').count()) await page.locator('[data-history-find-open]').click();
  await field.fill('needleOld');await submit();await page.locator('[data-history-find-hit]').filter({hasText: 'B row 50'}).waitFor();
  assert.equal(await page.locator('[data-history-find-hit]').filter({hasText: 'A row 50'}).count(), 0);
  assert.equal(await page.evaluate(() => window.__historyWire.clipboard.text), 'OWNER_BARRIER', 'retired copy must not deliver its old source text');
  passed('binding switch retires a held old copy without delivering it to the new reader');

  await field.fill('拼写');await field.dispatchEvent('keydown', {key: 'Enter', code: 'Enter', isComposing: true});
  await field.dispatchEvent('keydown', {key: 'Escape', code: 'Escape', isComposing: true});assert.equal(await field.inputValue(), '拼写');
  await page.setViewportSize({width: 320, height: 700});
  assert(await page.locator('[data-history-search]').evaluate(node => node.getBoundingClientRect().right <= innerWidth + 1));
  await page.screenshot({path: join(evidence, 'narrow.png')});
  passed('narrow controls remain reachable and IME Enter/Escape preserve the draft');

  for (const mode of ['document', 'zero-height']) {
    await page.evaluate(mode => {window.__historyWire.mode = mode;window.__historyWire.capture('mode-start');}, mode);
    await field.fill('absent');
    await page.evaluate(() => {window.__historyWire.holdInitial = true;});await submit();
    await until(() => page.evaluate(() => !!window.__historyWire.held), 'empty search has a real held source read');
    const heldKey = await page.evaluate(() => window.__historyWire.heldKey);
    assert(heldKey, 'held read has an exact request identity');
    await page.evaluate(() => window.__historyWire.capture('before-hide'));
    if (mode === 'document') await page.evaluate(() => window.__historyWire.hidden(true));
    else await page.evaluate(() => window.__probe.hide(true));
    await until(() => page.locator('[data-history-search]').count().then(count => count === 0), 'hidden search closes');
    await page.evaluate(() => window.__historyWire.capture('search-closed'));
    await until(() => page.evaluate(key => {
      const wire = window.__historyWire, held = wire.leases[key];
      return held?.claimed && held.cancelled && Object.values(wire.leases).every(lease => lease.cancelled);
    }, heldKey), 'cancel reaches the exact held IO without releasing its slot');
    assert.equal(await page.evaluate(key => window.__historyWire.calls.filter(call => call.cmd === 'history_cancel' && call.args.requestKey === key).length, heldKey), 1, 'held read is cancelled once');
    const count = await page.evaluate(() => window.__historyWire.calls.filter(call => call.cmd === 'history_page').length);
    // Deliberately deliver the late response to exercise the real source's
    // stale-answer guard. The claimed ledger retires only when this IO settles.
    await page.evaluate(() => window.__historyWire.release());
    await until(() => page.evaluate(() => Object.keys(window.__historyWire.leases).length === 0), 'finished IO drops all cancelled request slots');
    await page.waitForTimeout(100);
    assert.equal(await page.evaluate(() => window.__historyWire.calls.filter(call => call.cmd === 'history_page').length), count, 'old empty response must not continue scanning while hidden');
    if (mode === 'document') {
      await page.evaluate(() => window.__historyWire.hidden(false));
      await page.locator('[data-history-find-open]').click();
    }
  }
  await page.evaluate(() => {window.__historyWire.capture('before-unmount');window.__probe.show(false);});
  await page.waitForTimeout(100);
  assert.deepEqual(await page.evaluate(() => window.__historyWire.leases), {}, 'unmount leaves no request slots');
  assert.deepEqual(await page.evaluate(() => window.__historyWire.unknown), []);
  assert.deepEqual(errors, []);
  passed('existing visibility and unmount retire worker and source leases');
} catch (error) {
  if (page) await page.screenshot({path: join(evidence, 'failure.png')}).catch(() => {});
  throw error;
} finally {
  const stats = page ? await page.evaluate(() => ({calls: window.__historyWire?.calls, unknown: window.__historyWire?.unknown, leases: window.__historyWire?.leases, jobs: window.__historyWire?.jobs, clipboard: window.__historyWire?.clipboard, trace: window.__historyWire?.trace})).catch(() => null) : null;
  const cleanup = await Promise.allSettled([browser?.close(), server?.close()]);rmSync(fixture, {recursive: true, force: true});
  const after = fingerprint();writeFileSync(join(evidence, 'source-after.json'), JSON.stringify(after, null, 2));
  writeFileSync(join(evidence, 'report.json'), JSON.stringify({results, errors, stats, cleanup: cleanup.map(row => row.status), stable: JSON.stringify(before) === JSON.stringify(after), fixtureRemoved: !readdirSync(gui).includes(fixture.split('/').at(-1)), scope: 'production UI/worker; API and clipboard sink only; not actual WK/OS Save or device touch'}, null, 2));
  assert(cleanup.every(row => row.status === 'fulfilled'), 'owned browser/server cleanup failed');
  assert.deepEqual(after, before, 'source/gen changed during browser verification');
}
