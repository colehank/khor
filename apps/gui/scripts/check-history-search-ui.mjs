// Actual production ChatView/worker/reader/hook. Only external API/clipboard IO is stubbed.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync, mkdirSync, readFileSync, writeFileSync, readdirSync, rmSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {createServer} from 'vite';
import {chromium} from 'playwright-core';

const gui = fileURLToPath(new URL('..', import.meta.url));
const rangeCapOnly=process.argv.includes('--range-cap-only');
const rangeOnly=rangeCapOnly || process.argv.includes('--range-only');
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
    clipboard: {text: '', writes: 0, rejected: 0, activation: false}, order: [], holdOlder: false,
    anchorText: Array.from({length:350},(_,i)=>`Retained contribution word${i} carries the same visible paragraph text.`).join(' '),
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
      if(args.binding==='bind-cap-anchor'){
        const text=i=>`Cap retained contribution entry${i} carries${i} stable${i} visible${i} words${i} across${i} the${i} bounded${i} history${i} window${i}. These${i} are${i} the${i} same${i} original${i} source${i} words${i} before${i} and${i} after${i} the${i} older${i} page${i} arrives${i}.\n\n`;
        wire.anchorText=text(200).trim();
        const scope='cap:bind-cap-anchor:', at=args.cursor===null?640:Number(args.cursor.slice(scope.length));
        if(args.cursor!==null && (!args.cursor.startsWith(scope) || !Number.isInteger(at) || at<0 || at>640))throw new Error('invalid fixed cap cursor');
        const start=args.newer?at:Math.max(0,at-128),end=args.newer?Math.min(640,at+128):at;
        const entries=Array.from({length:end-start},(_,offset)=>start+offset).map(index=>note(`cap-entry-${index}`,text(index),
          {start:index*1000,end:(index+1)*1000,item:0,cursor:scope+index,missing_start:false,missing_end:false},false));
        const selected={entries,references:[],older:start?scope+start:null,newer:scope+end,has_more:args.newer && end<640,reset:false,encoded_bytes:0};
        selected.encoded_bytes=encoder.encode(JSON.stringify(selected)).length+20;
        if(wire.holdOlder && !args.newer && at===128){wire.holdOlder=false;wire.heldKey=args.requestKey;
          return new Promise(resolve=>{wire.held=()=>resolve(selected);}).finally(()=>{delete wire.leases[args.requestKey];});}
        delete wire.leases[args.requestKey];return selected;
      }
      if(args.binding.startsWith('bind-anchor-')){
        const scope=`anchor:${args.binding}:`, between=scope+'between', tail=scope+'snapshot';
        const prefix=args.binding.endsWith('-late')
          ? {key:'anchor-prefix-control',starts_message:true,record:{start:0,end:10000,item:0,cursor:scope+'before-prefix',missing_start:false,missing_end:false},frame:{kind:'turn',stop:'EndTurn'}}
          : note('anchor-prefix','Older prefix reflows this retained paragraph. '.repeat(80)+'**prefix emphasis** ',
            {start:0,end:10000,item:0,cursor:scope+'before-prefix',missing_start:false,missing_end:false},false);
        const retained=note('anchor-retained',wire.anchorText,
          {start:10000,end:60000,item:0,cursor:scope+'before-retained',missing_start:false,missing_end:false},false);
        let selected;
        if(args.cursor===null || (args.newer && (args.cursor===between || args.cursor===scope+'before-retained')))
          selected={entries:[retained],references:[],older:between,newer:tail,has_more:false,reset:false,encoded_bytes:0};
        else if((!args.newer && (args.cursor===between || args.cursor===scope+'before-retained')) || (args.newer && args.cursor===scope+'before-prefix'))
          selected={entries:[prefix],references:[],older:null,newer:between,has_more:!!args.newer,reset:false,encoded_bytes:0};
        else if(args.newer && args.cursor===tail)
          selected={entries:[],references:[],older:between,newer:tail,has_more:false,reset:false,encoded_bytes:0};
        else throw new Error('unknown fixed anchor source cursor');
        selected.encoded_bytes=encoder.encode(JSON.stringify(selected)).length+20;
        if(wire.holdOlder && !args.newer && args.cursor===between){
          wire.holdOlder=false;wire.heldKey=args.requestKey;
          return new Promise(resolve=>{wire.held=()=>resolve(selected);}).finally(()=>{delete wire.leases[args.requestKey];});
        }
        delete wire.leases[args.requestKey];return selected;
      }
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
      if ((wire.holdCopy && args.newer) || (wire.holdInitial && !args.newer && args.cursor === null)) {
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
  await page.waitForFunction(()=>!!window.__probe);
  const words = await page.evaluate(() => window.__probe.words());
  if(!rangeOnly){
  await page.locator('[data-history-find-open]').click();
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
  }

  // One existing service/real ChatView: the older SDK chunk extends the same
  // Markdown paragraph, so card-bottom preservation is not the text oracle.
  await page.setViewportSize({width:1100,height:800});
  const textPoint = (saved = null) => {
    const view=document.querySelector('[data-chat-scroll]');
    const root=[...document.querySelectorAll('[data-history-key]')].find(node=>node.textContent.includes(window.__historyWire.anchorText));
    if(!view || !root)throw new Error('retained contribution is not mounted');
    const body=window.__historyWire.anchorText, sourceStart=root.textContent.indexOf(body);
    if(sourceStart<0)throw new Error('retained contribution text is no longer exact');
    const bounds=root.getBoundingClientRect(), viewport=view.getBoundingClientRect();
    let offset=saved?.offset;
    if(offset===undefined){
      const line=parseFloat(getComputedStyle(root).lineHeight);
      const x=(Math.max(bounds.left,viewport.left)+Math.min(bounds.right,viewport.right))/2;
      const top=Math.max(bounds.top,viewport.top), bottom=Math.min(bounds.bottom,viewport.bottom);
      const y=Math.min(bottom,top+(Number.isFinite(line)?line/2:(bottom-top)/2));
      const caret=document.caretPositionFromPoint?.(x,y);
      const range=caret?document.createRange():document.caretRangeFromPoint?.(x,y);
      if(caret){range.setStart(caret.offsetNode,caret.offset);range.collapse(true);}
      if(!range || !root.contains(range.startContainer) || range.startContainer.nodeType!==Node.TEXT_NODE)throw new Error('no visible retained text caret');
      const walker=document.createTreeWalker(root,NodeFilter.SHOW_TEXT);let at=0;
      while(walker.nextNode()){if(walker.currentNode===range.startContainer){at+=range.startOffset;break;}at+=walker.currentNode.length;}
      offset=at-sourceStart;
    }
    if(offset<0 || offset>=body.length)throw new Error('caret is outside the retained contribution');
    const walker=document.createTreeWalker(root,NodeFilter.SHOW_TEXT);let at=sourceStart+offset, selected;
    while(walker.nextNode()){const node=walker.currentNode;if(at<node.length){selected=document.createRange();selected.setStart(node,at);selected.setEnd(node,at+1);break;}at-=node.length;}
    const rect=selected?.getClientRects()[0];if(!rect?.height)throw new Error('retained text range has no geometry');
    return {offset,character:body.slice(offset,offset+1),sample:body.slice(Math.max(0,offset-24),offset+24),
      top:rect.top-viewport.top,key:root.dataset.historyKey,scrollTop:view.scrollTop,height:bounds.height};
  };
  for(const lateInput of rangeCapOnly?[]:[false,true]){
    await page.evaluate(lateInput=>{
      window.__historyWire.mode=lateInput?'anchor-newer-input':'anchor-text-point';
      window.__probe.hide(false);window.__probe.change(lateInput?'bind-anchor-late':'bind-anchor-point');window.__probe.show(true);
    },lateInput);
    await page.locator('[data-history-key]').filter({hasText:'Retained contribution word0'}).waitFor();
    await page.evaluate(()=>document.fonts.ready);
    const scroll=page.locator('[data-chat-scroll]');
    await until(()=>scroll.evaluate(node=>node.scrollTop>550 && Math.abs(node.scrollHeight-node.clientHeight-node.scrollTop)<3),'initial long source is settled at latest');
    const bounds=await scroll.boundingBox();assert(bounds);
    await page.mouse.move(bounds.x+bounds.width/2,bounds.y+bounds.height/2);
    await page.mouse.wheel(0,-300);await page.waitForTimeout(80);
    const original=await page.evaluate(textPoint);
    assert(original.scrollTop>250,'the long real body has room for a subsequent genuine reading input');
    await page.evaluate(()=>{window.__historyWire.holdOlder=true;});
    const older=page.getByRole('button',{name:words.history_older,exact:true});
    await older.evaluate(node=>node.focus({preventScroll:true}));await page.keyboard.press('Enter');
    await until(()=>page.evaluate(()=>!!window.__historyWire.held),'older page is held after the production anchor capture');
    let expected=original;
    if(lateInput){
      await page.mouse.wheel(0,-240);await page.waitForTimeout(80);
      expected=await page.evaluate(textPoint);
      assert(expected.offset!==original.offset && Math.abs(expected.scrollTop-original.scrollTop)>100,'newer user reading position is distinct from the old capture');
    }
    const beforeRO=await page.evaluate(()=>performance.now());
    await page.evaluate(()=>window.__historyWire.release());
    if(lateInput)await until(()=>older.count().then(count=>count===0),'older control page commits without changing the retained body');
    else await page.locator('[data-history-key]').filter({hasText:'prefix emphasis'}).waitFor();
    await until(()=>page.evaluate(mark=>window.__historyWire.trace.some(row=>row.event==='resize-after' && row.at>mark),beforeRO),'existing observers receive the paragraph reflow');
    const samples=[];
    for(let frame=0;frame<6;frame++){
      await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
      samples.push(await page.evaluate(textPoint,expected));
    }
    await page.waitForTimeout(120);samples.push(await page.evaluate(textPoint,expected));
    for(const actual of samples){
      assert.equal(actual.sample,expected.sample);assert.equal(actual.character,expected.character);
      assert.equal(actual.key,expected.key,'the retained contribution keeps its disclosure owner');
      assert(Math.abs(actual.top-expected.top)<=2,`${lateInput?'newer input':'same text point'} moved after settled Range restore: ${JSON.stringify({expected,actual})}`);
    }
    if(!lateInput)assert(samples[0].height>original.height,'the older prefix really extends/reflows the same painted paragraph');
    await page.evaluate(()=>window.__probe.show(false));
    await until(()=>page.evaluate(()=>Object.keys(window.__historyWire.leases).length===0),'anchor scene releases its exact IO leases');
  }
  // Fill the real 512-entry window first; the next older page evicts the
  // newer tail, while source entry200 and its exact visible text point remain.
  await page.evaluate(()=>{window.__historyWire.mode='anchor-cap-eviction';window.__probe.change('bind-cap-anchor');window.__probe.show(true);});
  await page.locator('[data-history-key]').filter({hasText:'Cap retained contribution entry512 '}).waitFor();
  const capOlder=page.getByRole('button',{name:words.history_older,exact:true});
  for(const first of [384,256,128]){
    await capOlder.evaluate(node=>node.focus({preventScroll:true}));await page.keyboard.press('Enter');
    await page.locator('[data-history-key]').filter({hasText:`Cap retained contribution entry${first} `}).waitFor();
    await until(()=>capOlder.isEnabled(),'filled page commit completes');
  }
  assert.equal(await page.locator('[data-history-key]').filter({hasText:'Cap retained contribution entry639 '}).count(),1,'newer tail is present before the capped prepend');
  assert.equal(await page.locator('[data-history-key]').evaluateAll(nodes=>nodes.reduce((sum,node)=>sum+(node.textContent.match(/Cap retained contribution entry[0-9]+ /g)?.length??0),0)),512,'one painted body really fills the bounded entry window');
  const capScroll=page.locator('[data-chat-scroll]'),capBounds=await capScroll.boundingBox();assert(capBounds);
  await page.mouse.move(capBounds.x+capBounds.width/2,capBounds.y+capBounds.height/2);
  const delta=await page.evaluate(()=>{
    const view=document.querySelector('[data-chat-scroll]'),needle=window.__historyWire.anchorText;
    const walker=document.createTreeWalker(view,NodeFilter.SHOW_TEXT);
    while(walker.nextNode()){const at=walker.currentNode.data.indexOf(needle);if(at<0)continue;
      const range=document.createRange();range.setStart(walker.currentNode,at);range.setEnd(walker.currentNode,at+1);
      return range.getClientRects()[0].top-view.getBoundingClientRect().top;}
    throw new Error('retained cap contribution has no text range');
  });
  await page.mouse.wheel(0,delta);await page.waitForTimeout(100);
  const capPoint=await page.evaluate(textPoint);
  await page.evaluate(()=>{window.__historyWire.holdOlder=true;});
  await capOlder.evaluate(node=>node.focus({preventScroll:true}));await page.keyboard.press('Enter');
  await until(()=>page.evaluate(()=>!!window.__historyWire.held),'capped older prepend has a real held read');
  const capRO=await page.evaluate(()=>performance.now());await page.evaluate(()=>window.__historyWire.release());
  await page.locator('[data-history-key]').filter({hasText:'Cap retained contribution entry0 '}).waitFor();
  assert.equal(await page.locator('[data-history-key]').filter({hasText:'Cap retained contribution entry639 '}).count(),0,'the newer tail actually leaves the bounded window');
  assert.equal(await page.locator('[data-history-key]').filter({hasText:'Cap retained contribution entry512 '}).count(),0);
  assert.equal(await page.locator('[data-history-key]').evaluateAll(nodes=>nodes.reduce((sum,node)=>sum+(node.textContent.match(/Cap retained contribution entry[0-9]+ /g)?.length??0),0)),512,'the older prepend preserves the same bounded window size');
  await until(()=>page.evaluate(mark=>window.__historyWire.trace.some(row=>row.event==='resize-after' && row.at>mark),capRO),'cap eviction settles through the existing observer');
  const capSamples=[];
  for(let frame=0;frame<6;frame++){
    await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    capSamples.push(await page.evaluate(textPoint,capPoint));
  }
  await page.waitForTimeout(120);capSamples.push(await page.evaluate(textPoint,capPoint));
  for(const actual of capSamples){assert.equal(actual.sample,capPoint.sample);assert.equal(actual.key,capPoint.key);
    assert(Math.abs(actual.top-capPoint.top)<=2,`CAP_EVICTION_TEXT_POINT: retained source point moved: ${JSON.stringify({expected:capPoint,actual})}`);}
  await page.evaluate(()=>window.__probe.show(false));
  await until(()=>page.evaluate(()=>Object.keys(window.__historyWire.leases).length===0),'capped anchor scene releases its leases');
  assert.deepEqual(await page.evaluate(()=>window.__historyWire.unknown),[]);
  assert.deepEqual(errors,[]);
  passed(rangeCapOnly ? 'retained text Range survives capped newer-tail eviction across RAF and observer settle' : 'retained text Range survives older paragraph reflow and capped tail eviction across settle; newer user input wins');

} catch (error) {
  if (page) await page.screenshot({path: join(evidence, 'failure.png')}).catch(() => {});
  throw error;
} finally {
  const stats = page ? await page.evaluate(() => ({calls: window.__historyWire?.calls, unknown: window.__historyWire?.unknown, leases: window.__historyWire?.leases, jobs: window.__historyWire?.jobs, clipboard: window.__historyWire?.clipboard, trace: window.__historyWire?.trace})).catch(() => null) : null;
  const cleanup = await Promise.allSettled([browser?.close(), server?.close()]);rmSync(fixture, {recursive: true, force: true});
  const after = fingerprint();writeFileSync(join(evidence, 'source-after.json'), JSON.stringify(after, null, 2));
  writeFileSync(join(evidence, 'report.json'), JSON.stringify({results, errors, stats, cleanup: cleanup.map(row => row.status), stable: JSON.stringify(before) === JSON.stringify(after), fixtureRemoved: !readdirSync(gui).includes(fixture.split('/').at(-1)), scope: 'production UI/worker; API and clipboard sink only; not actual WK/OS Save or device touch',selection:{rangeOnly,rangeCapOnly}}, null, 2));
  assert(cleanup.every(row => row.status === 'fulfilled'), 'owned browser/server cleanup failed');
  assert.deepEqual(after, before, 'source/gen changed during browser verification');
}
