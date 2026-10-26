import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {existsSync, mkdtempSync, readFileSync, writeFileSync, mkdirSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium, webkit} from 'playwright-core';
import {nativeEventRuntime} from './fixtures/native-events/runtime.mjs';

const guiUrl = new URL('../', import.meta.url), gui = fileURLToPath(guiUrl);
const known = ['page-late', 'page-live', 'page-ready-close', 'page-closed-before-import', 'fullscreen-late', 'fullscreen-query', 'quit-late', 'quit-prevent', 'quit-destroy', 'quit-close-in-handler', 'quit-destroy-rejected', 'window-drop-late', 'window-drop-partial', 'drag-payloads', 'drag-close-during-admission', 'drag-partial', 'drag-unlisten-rejected', 'sdk-drag-payloads', 'callback-throws-peer'];
const scenarios = process.env.NATIVE_HELPERS_SCENARIOS?.split(',') ?? known;
assert(scenarios.length && new Set(scenarios).size === scenarios.length);
for (const name of scenarios) assert(known.includes(name), `Unknown scenario ${name}`);
const sources = ['src/api.ts', 'src/lib/native-events.ts', 'src/lib/window-drop.ts', 'src/views/ChatView.tsx', 'package-lock.json', '../../Cargo.lock',
  ...['core', 'event', 'window', 'webview', 'dpi'].map(name => `node_modules/@tauri-apps/api/${name}.js`),
  'scripts/check-native-helpers.mjs', 'scripts/fixtures/native-events/runtime.mjs', 'scripts/fixtures/native-events/helpers-probe.ts',
  ...['core.js', 'event-mod.rs', 'event-init.js'].map(file => `scripts/fixtures/native-events/tauri-2.11.5/${file}`)];
const fingerprint = () => Object.fromEntries(sources.map(file => [file, existsSync(join(gui, file)) ? createHash('sha256').update(readFileSync(join(gui, file))).digest('hex') : null]));
const sourceBefore = fingerprint(), results = [];
const bootstrap = nativeEventRuntime(guiUrl) + '\nwindow.__TAURI_INTERNALS__.metadata = {currentWindow: {label: "window-alpha"}, currentWebview: {label: "webview-alpha"}};';
const evidence = process.env.NATIVE_HELPERS_CHECK_DIR || mkdtempSync(join(tmpdir(), 'khor-native-helpers-'));
mkdirSync(evidence, {recursive: true});
const fixture = mkdtempSync(join(gui, '.native-helper-check-'));
let server, browser;
async function exercise(page, name) {
  const act = (action, ...args) => page.evaluate(({action, args}) => window.__helperProbe[action](...args), {action, args});
  const state = () => act('state');
  const turn = () => page.evaluate(() => new Promise(resolve => setTimeout(resolve, 0)));
  const settle = async (fail = false) => { await turn(); await act('settleUnlisten', fail); await turn(); };
  const listens = async count => {
    await page.waitForFunction(count => window.__helperProbe.state().messages.filter(m => m.cmd === 'plugin:event|listen').length >= count, count);
    return (await state()).messages.filter(m => m.cmd === 'plugin:event|listen');
  };
  const finish = async () => { await act('close'); await act('close'); await settle(); return state(); };
  if (name === 'page-closed-before-import') {
    await page.evaluate(() => { window.__helperProbe.mount('page'); window.__helperProbe.close(); });
    await turn(); await turn();
    const actual = await state(); assert.equal(actual.messages.length, 0, 'Closed owner must not allocate a listener'); return actual;
  }
  const mode = name === 'sdk-drag-payloads' ? 'sdk-drag' : name === 'callback-throws-peer' ? 'peer' : name.startsWith('page') ? name === 'page-ready-close' ? name : 'page' : name.startsWith('fullscreen') ? 'fullscreen' : name.startsWith('window-drop') ? 'window-drop' : name.startsWith('drag') ? 'drag' : name === 'quit-destroy-rejected' ? 'quit-destroy' : name;
  await act('mount', mode);
  let messages = await listens(1);
  const expectedTarget = mode === 'page' || mode === 'page-ready-close' || mode === 'peer' ? {kind: 'Any'} : mode === 'window-drop' || mode === 'drag' || mode === 'sdk-drag' ? {kind: 'Webview', label: 'webview-alpha'} : {kind: 'Window', label: 'window-alpha'};
  assert.deepEqual(messages[0].target, expectedTarget, 'Preserve the SDK event target exactly');
  if (name === 'callback-throws-peer') {
    messages = await listens(2);
    for (const message of messages) await act('accept', message.index);
    await turn(); await act('emit', 'q043:helpers-peer', null);
    const actual = await finish(); assert.deepEqual(actual.received, ['peer']); assert.equal(actual.callbacks, 0); assert.deepEqual(actual.native, []); return actual;
  }
  if (name.endsWith('-late')) {
    await act('close');
    const released = (await state()).handlers[0];
    const id = await act('accept', messages[0].index, false); await turn(); await act('register', messages[0].index, id);
    await act('emit', messages[0].event, name.startsWith('page') ? {tab: 'alpha', text: 'closed'} : {paths: ['closed'], position: {x: 1, y: 2}});
    if (name.startsWith('fullscreen')) {
      const query = (await state()).messages.find(m => m.cmd === 'plugin:window|is_fullscreen');
      if (query) await act('reply', query.index, true);
    }
    const actual = await finish(); assert.equal(released, false, 'Close must synchronously release the JS callback');
    assert.deepEqual(actual.received, []); assert.equal(actual.ready, 0); assert.equal(actual.callbacks, 0); assert.deepEqual(actual.native, []); return actual;
  }
  if (name.startsWith('page')) {
    await act('accept', messages[0].index); await turn();
    await act('emit', 'page:title', {tab: 'other', text: 'wrong'});
    await act('emit', 'page:title', {tab: 'alpha', text: 'correct'});
    const live = await state(); assert.equal(live.ready, 1); assert.deepEqual(live.received, name === 'page-ready-close' ? [] : ['correct']);
    const actual = await finish(); assert.equal(actual.callbacks, 0); assert.deepEqual(actual.native, []); return actual;
  }
  if (name === 'fullscreen-query') {
    await act('accept', messages[0].index); await turn();
    let queries = (await state()).messages.filter(m => m.cmd === 'plugin:window|is_fullscreen'); assert.equal(queries.length, 1);
    await act('reply', queries[0].index, true); await turn();
    await act('emit', 'tauri://resize', {width: 100, height: 100}); await turn();
    queries = (await state()).messages.filter(m => m.cmd === 'plugin:window|is_fullscreen'); assert.equal(queries.length, 2);
    await act('close'); await act('reply', queries[1].index, false); await act('emit', 'tauri://resize', {width: 200, height: 200});
    const actual = await finish(); assert.deepEqual(actual.received, [true]); assert.equal(actual.messages.filter(m => m.cmd === 'plugin:window|is_fullscreen').length, 2); assert.equal(actual.callbacks, 0); assert.deepEqual(actual.native, []); return actual;
  }
  if (name.startsWith('quit')) {
    await act('accept', messages[0].index); await turn(); await act('emit', 'tauri://close-requested', null); await turn();
    let actual = await state(); assert.deepEqual(actual.received, ['asked']);
    const destroy = actual.messages.filter(m => m.cmd === 'plugin:window|destroy');
    assert.equal(destroy.length, name === 'quit-prevent' || name === 'quit-close-in-handler' ? 0 : 1, 'Await handler and respect prevent/owner before destroy');
    for (const message of destroy) await act('reply', message.index, name === 'quit-destroy-rejected' ? 'fixture destroy denied' : null, name === 'quit-destroy-rejected');
    actual = await finish(); assert.equal(actual.callbacks, 0); assert.deepEqual(actual.native, []); return actual;
  }
  if (name.includes('partial')) {
    await act('accept', messages[0].index); messages = await listens(2); await act('accept', messages[1].index); messages = await listens(3); await act('reject', messages[2].index); await settle();
    await act('emit', 'tauri://drag-enter', {paths: ['/closed'], position: {x: 1, y: 2}});
    const rolled = await state(); assert.equal(rolled.callbacks, 0, 'Partial failure must free all allocated JS callbacks'); assert.deepEqual(rolled.native, [], 'Partial failure must roll back earlier native admissions'); assert.deepEqual(rolled.received, []);
    return finish();
  }
  if (name === 'drag-close-during-admission') {
    await act('accept', messages[0].index); messages = await listens(2); await act('close'); const id = await act('accept', messages[1].index, false); await turn(); await act('register', messages[1].index, id); await settle();
    const actual = await finish(); assert.equal(actual.messages.filter(m => m.cmd === 'plugin:event|listen').length, 2); assert.equal(actual.callbacks, 0); assert.deepEqual(actual.native, []); return actual;
  }
  for (let i = 0; i < 4; i++) { messages = await listens(i + 1); assert.deepEqual(messages[i].target, expectedTarget); await act('accept', messages[i].index); }
  await turn();
  for (const event of ['enter', 'over', 'drop', 'leave']) await act('emit', `tauri://drag-${event}`, {paths: ['/file'], position: {x: 12, y: 34}});
  const live = await state(); assert.deepEqual(live.received, [
    {type: 'enter', paths: ['/file'], position: {type: "Physical", x: 12, y: 34}, physical: true},
    {type: 'over', position: {type: "Physical", x: 12, y: 34}, physical: true},
    {type: 'drop', paths: ['/file'], position: {type: "Physical", x: 12, y: 34}, physical: true},
    {type: 'leave', physical: null},
  ]);
  await act('close'); await settle(name === 'drag-unlisten-rejected');
  await act('emit', 'tauri://drag-drop', {paths: ['/closed'], position: {x: 12, y: 34}});
  const actual = await state(); assert.deepEqual(actual.received, live.received); assert.equal(actual.callbacks, 0); assert.equal(actual.native.length, name === 'drag-unlisten-rejected' ? 4 : 0); return actual;
}
try {
  writeFileSync(join(fixture, 'bootstrap.js'), bootstrap);
  writeFileSync(join(fixture, 'index.html'), '<link rel="icon" href="data:,"><script src="./bootstrap.js"></script><script type="module" src="./probe.ts"></script>');
  writeFileSync(join(fixture, 'probe.ts'), readFileSync(new URL('./fixtures/native-events/helpers-probe.ts', import.meta.url)));
  process.env.KHOR_FACE_GATE = '1';
  server = await createServer({root: gui, cacheDir: join(fixture, '.vite'), optimizeDeps: {entries: [join(fixture, 'index.html')]}, server: {host: '127.0.0.1', port: Number(process.env.NATIVE_HELPERS_PORT || 23844), strictPort: true, hmr: false}, logLevel: 'error'});
  await server.listen();
  for (const [engine, name] of [[chromium, 'chrome'], [webkit, 'webkit']]) {
    browser = await engine.launch(name === 'chrome' ? {channel: (process.env.PLAYWRIGHT_CHROMIUM_CHANNEL ?? 'chrome') || undefined, headless: true} : {headless: true});
    for (const scenario of scenarios) {
      const page = await browser.newPage(); page.setDefaultTimeout(5000);
      const errors = [], diagnostics = [];
      page.on('pageerror', error => errors.push(error.message)); page.on('console', message => { if (message.type() === 'error') diagnostics.push(message.text()); });
      try {
        await page.goto(`http://127.0.0.1:${server.config.server.port}/${fixture.split('/').pop()}/index.html`); await page.waitForFunction(() => window.__helperReady);
        const actual = await exercise(page, scenario);
        assert.deepEqual(errors, [], 'No unhandled runtime error'); assert.deepEqual(actual.errors, [], 'No unhandled Promise rejection');
        if (scenario.includes('partial')) assert(diagnostics.some(m => /listen/.test(m)), 'Partial refusal must be diagnosed');
        else if (scenario === 'callback-throws-peer') assert(diagnostics.some(m => /handler/.test(m)), 'A failing handler must be diagnosed without stopping its peer');
        else if (scenario.includes('rejected')) assert(diagnostics.some(m => /failed/.test(m)), 'Cleanup/destroy refusal must be diagnosed');
        else assert.deepEqual(diagnostics, []);
        results.push({engine: name, scenario, ok: true, actual, diagnostics}); console.log('PASS', name, scenario);
      } catch (error) {
        results.push({engine: name, scenario, ok: false, error: error.message, errors, diagnostics, actual: await page.evaluate(() => window.__helperProbe?.state()).catch(() => null)}); console.error('FAIL', name, scenario, error.message);
      } finally { await page.close(); }
    }
    await browser.close(); browser = null;
  }
  const sourceAfter = fingerprint(); assert.deepEqual(sourceAfter, sourceBefore, 'Source must remain frozen');
  writeFileSync(join(evidence, 'report.json'), JSON.stringify({sourceBefore, sourceAfter, source_stable: true, results}, null, 2));
  assert(results.every(result => result.ok), `${results.filter(result => !result.ok).length} native-helper regressions failed`); console.log(`native helpers: ${results.length}/${results.length} passed`, evidence);
} finally { await browser?.close(); await server?.close(); rmSync(fixture, {recursive: true, force: true}); }
