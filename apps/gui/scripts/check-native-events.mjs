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
const fixture = mkdtempSync(join(gui, '.native-event-check-'));
const evidence = process.env.NATIVE_EVENTS_CHECK_DIR || mkdtempSync(join(tmpdir(), 'khor-native-events-'));
mkdirSync(evidence, {recursive: true});
const known = ['late-existing-group', 'late-no-group', 'ready-before-eval', 'close-before-import', 'normal-live', 'listen-rejected', 'close-then-reject', 'unlisten-rejected', 'same-event-remount', 'peer-rejected', 'web-noop'];
const scenarios = process.env.NATIVE_EVENTS_SCENARIOS?.split(',') ?? known;
assert(scenarios.length && new Set(scenarios).size === scenarios.length);
for (const name of scenarios) assert(known.includes(name), `Unknown scenario ${name}`);
const sources = ['src/api.ts', 'src/lib/native-events.ts', 'package-lock.json', 'scripts/check-native-events.mjs',
  'scripts/fixtures/native-events/runtime.mjs', 'scripts/fixtures/native-events/probe.tsx',
  ...['core.js', 'event-mod.rs', 'event-init.js'].map(file => `scripts/fixtures/native-events/tauri-2.11.5/${file}`)];
const fingerprint = () => Object.fromEntries(sources.map(file => [file, existsSync(join(gui, file)) ? createHash('sha256').update(readFileSync(join(gui, file))).digest('hex') : null]));
const sourceBefore = fingerprint(), results = [];
writeFileSync(join(fixture, 'bootstrap.js'), nativeEventRuntime(guiUrl));
writeFileSync(join(fixture, 'index.html'), '<link rel="icon" href="data:,"><div id="root"></div><script src="./bootstrap.js"></script><script type="module" src="./probe.tsx"></script>');
writeFileSync(join(fixture, 'probe.tsx'), readFileSync(new URL('./fixtures/native-events/probe.tsx', import.meta.url)));
let server, browser;

async function exercise(page, name) {
  const act = (action, ...args) => page.evaluate(({action, args}) => window.__eventProbe[action](...args), {action, args});
  const state = () => act('state');
  const turn = () => page.evaluate(() => new Promise(resolve => setTimeout(resolve, 0)));
  const listenCount = count => page.waitForFunction(count => window.__eventProbe.state().messages.filter(m => m.cmd === 'plugin:event|listen').length === count, count);
  const settle = async () => { await turn(); await act('settleUnlisten'); await turn(); };
  const finish = async () => { await act('mount', []); await settle(); await act('closeAnchor'); await settle(); return state(); };
  if (name === 'web-noop' || name === 'close-before-import') {
    // Both effect setup and cleanup happen in one task, before a cached import can finish.
    await page.evaluate(() => { window.__eventProbe.mount(['alpha']); window.__eventProbe.mount([]); });
    await turn(); await turn();
    const actual = await state();
    assert.deepEqual(actual.messages, [], 'A closed owner must not start listening');
    assert.equal(actual.callbacks, 0); return actual;
  }
  const anchored = ['late-existing-group', 'ready-before-eval'].includes(name);
  if (anchored) await act('anchor');
  const offset = anchored ? 1 : 0;
  if (['same-event-remount', 'peer-rejected'].includes(name)) {
    if (name === 'same-event-remount') {
      await act('mount', ['alpha']); await listenCount(1);
      await act('mount', []);
      await act('mount', ['beta']); await listenCount(2);
      await act('accept', 1, 3, true); await turn();
      await act('accept', 0, 2); await turn(); await act('register', 0, 2);
      await act('emit', [2, 3]); await settle();
    } else {
      await act('mount', ['alpha']); await listenCount(1);
      await act('mount', ['alpha', 'beta']); await listenCount(2);
      await act('accept', 0, 2, true); await act('reject', 1); await turn();
      await act('emit', [2]);
    }
    const live = await state();
    assert.deepEqual(live.received, [name === 'same-event-remount' ? 'beta' : 'alpha'], 'Peer failure or late old ack must not affect the live owner');
    const actual = await finish();
    assert.equal(actual.callbacks, 0, 'Every JS callback must be released');
    assert.deepEqual(actual.native, [], 'Every accepted native listener must be released');
    return actual;
  }
  await act('mount', ['alpha']); await listenCount(offset + 1);
  if (['listen-rejected', 'close-then-reject'].includes(name)) {
    if (name === 'close-then-reject') {
      await act('mount', []);
      assert.equal((await state()).handlers[offset], false, 'Close must synchronously release the already allocated handler');
    }
    await act('reject', offset); await turn();
    const actual = await finish();
    assert.equal(actual.callbacks, 0, 'Listen refusal must not leak its JS callback');
    assert.deepEqual(actual.native, []); return actual;
  }
  if (['late-existing-group', 'late-no-group', 'ready-before-eval'].includes(name)) {
    if (name === 'ready-before-eval') { await act('accept', offset, 2); await turn(); }
    await act('mount', []);
    const callbackAfterClose = (await state()).handlers[offset];
    if (name !== 'ready-before-eval') { await act('accept', offset, 2); await turn(); }
    await act('register', offset, 2);
    await act('emit', [2]);
    await act('close', 'alpha'); await act('close', 'alpha');
    const actual = await finish();
    assert.equal(callbackAfterClose, false, 'Close must release the handler before late registration');
    assert.deepEqual(actual.received, [], 'In-flight events must not call a closed owner');
    assert.equal(actual.callbacks, 0, 'Late registration must not resurrect a callback');
    assert.deepEqual(actual.native, [], 'Native removal must not depend on the JS registry');
    assert.equal(actual.messages.filter(m => m.cmd === 'plugin:event|unlisten' && m.id === 2).length, 1, 'Close is idempotent');
    return actual;
  }
  await act('accept', offset, 2, true); await turn();
  await act('emit', [2]);
  assert.deepEqual((await state()).received, ['alpha']);
  await act('mount', []); await act('close', 'alpha'); await act('close', 'alpha');
  await act('emit', [2]); await turn();
  await act('settleUnlisten', name === 'unlisten-rejected'); await turn();
  const actual = await state();
  assert.deepEqual(actual.received, ['alpha']);
  assert.equal(actual.callbacks, 0, 'JS callback must be freed even when native cleanup fails');
  assert.deepEqual(actual.native, name === 'unlisten-rejected' ? [2] : [], 'Refused native cleanup must remain visible, never reported released');
  assert.equal(actual.messages.filter(m => m.cmd === 'plugin:event|unlisten').length, 1);
  return actual;
}

try {
  process.env.KHOR_FACE_GATE = '1';
  server = await createServer({root: gui, cacheDir: join(fixture, '.vite'), optimizeDeps: {entries: [join(fixture, 'index.html')]},
    server: {host: '127.0.0.1', port: Number(process.env.NATIVE_EVENTS_PORT || 23843), strictPort: true, hmr: false}, logLevel: 'error'});
  await server.listen();
  for (const [engine, name] of [[chromium, 'chrome'], [webkit, 'webkit']]) {
    browser = await engine.launch(name === 'chrome' ? {channel: (process.env.PLAYWRIGHT_CHROMIUM_CHANNEL ?? 'chrome') || undefined, headless: true} : {headless: true});
    for (const scenario of scenarios) {
      const page = await browser.newPage(); page.setDefaultTimeout(5000);
      const errors = [], diagnostics = [];
      page.on('pageerror', error => errors.push(error.message));
      page.on('console', message => { if (message.type() === 'error') diagnostics.push(message.text()); });
      try {
        await page.goto(`http://127.0.0.1:${server.config.server.port}/${fixture.split('/').pop()}/index.html${scenario === 'web-noop' ? '?web=1' : ''}`);
        await page.waitForFunction(() => window.__eventReady);
        const actual = await exercise(page, scenario);
        for (const message of actual.messages.filter(m => m.cmd === 'plugin:event|listen')) assert.deepEqual(message.target, {kind: 'Any'});
        for (const payload of actual.payloads) assert.equal(payload, null, 'hear forwards the payload, not the event envelope');
        assert.deepEqual(errors, [], 'No unhandled Promise rejection');
        assert.deepEqual(actual.errors, [], 'No ignored cleanup rejection');
        if (['listen-rejected', 'close-then-reject', 'peer-rejected'].includes(scenario)) assert(diagnostics.some(message => /listen|subscrib/.test(message)), 'Listen failure has a diagnostic');
        else if (scenario === 'unlisten-rejected') assert(diagnostics.some(message => /unlisten|unsubscrib/.test(message)), 'Native cleanup failure has a diagnostic');
        else assert.deepEqual(diagnostics, []);
        results.push({engine: name, scenario, ok: true, actual, diagnostics}); console.log('PASS', name, scenario);
      } catch (error) {
        results.push({engine: name, scenario, ok: false, error: error.message, errors, diagnostics,
          actual: await page.evaluate(() => window.__eventProbe?.state()).catch(() => null)});
        console.error('FAIL', name, scenario, error.message);
      } finally { await page.close(); }
    }
    await browser.close(); browser = null;
  }
  const sourceAfter = fingerprint();
  assert.deepEqual(sourceAfter, sourceBefore, 'Source must remain frozen');
  writeFileSync(join(evidence, 'report.json'), JSON.stringify({sourceBefore, sourceAfter, source_stable: true, results}, null, 2));
  assert(results.every(result => result.ok), `${results.filter(result => !result.ok).length} native-event regressions failed`);
  console.log(`native events: ${results.length}/${results.length} passed`, evidence);
} finally { await browser?.close(); await server?.close(); rmSync(fixture, {recursive: true, force: true}); }
