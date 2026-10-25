// Run the production hook in real React. Only transport, time and the store
// notification are controlled; no app server, user store or native build.
import assert from 'node:assert/strict';
import { build } from 'esbuild';
import { chromium } from 'playwright-core';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const gui = fileURLToPath(new URL('..', import.meta.url));
const source = new URL('../src/hooks/use-lock.ts', import.meta.url);
const fingerprint = () => createHash('sha256').update(readFileSync(source)).digest('hex');
const beforeHash = fingerprint();
const bundle = await build({
  absWorkingDir: gui, write: false, bundle: true, format: 'iife', platform: 'browser',
  define: { 'process.env.NODE_ENV': '"development"' },
  stdin: { resolveDir: gui, loader: 'jsx', contents: `
    import React, { act } from 'react';
    import { createRoot } from 'react-dom/client';
    import { useLock } from './src/hooks/use-lock';
    const reads = [], pins = [], outcomes = [], history = [], intervals = new Map();
    let clock = 1000, timer = 0, current, root;
    Date.now = () => clock;
    window.setInterval = (run, ms) => { const id = ++timer; intervals.set(id, { run, ms, at: clock + ms }); return id; };
    window.clearInterval = id => intervals.delete(id);
    const deferred = into => new Promise((resolve, reject) => into.push({ resolve, reject }));
    window.__lockTransport = { fetchLock: () => deferred(reads), unlock: () => deferred(pins) };
    window.IS_REACT_ACT_ENVIRONMENT = true;
    function Host() { current = useLock(); history.push(snapshot()); return null; }
    function snapshot() { return { facts: current.facts, settled: current.settled, locked: current.locked }; }
    window.__lockTest = {
      async mount(strict) { root = createRoot(document.getElementById('root')); await act(async () => root.render(strict ? <React.StrictMode><Host/></React.StrictMode> : <Host/>)); },
      read: () => ({ ...snapshot(), reads: reads.length, pins: pins.length, outcomes, history, intervals: intervals.size }),
      async run(op, args = []) {
        await act(async () => {
          if (op === 'read') reads[args[0]].resolve(args[1]);
          else if (op === 'failRead') reads[args[0]].reject(new Error('offline'));
          else if (op === 'reread') current.reread();
          else if (op === 'notify') window.dispatchEvent(new StorageEvent('storage', { key: args[0] ?? 'khor.told.lock' }));
          else if (op === 'now') current.now();
          else if (op === 'opened') current.opened();
          else if (op === 'open') {
            const index = pins.length;
            void current.open('fixture-pin').then(value => { outcomes[index] = { value }; }, error => { outcomes[index] = { error: String(error) }; });
          }
          else if (op === 'pin') pins[args[0]].resolve(args[1]);
          else if (op === 'failPin') pins[args[0]].reject(new Error('verification unavailable'));
          else if (op === 'supersede') {
            // A newer read is requested before React runs effect cleanup.
            current.reread();
            if (args[1] === null) reads[args[0]].reject(new Error('old offline'));
            else reads[args[0]].resolve(args[1]);
            await Promise.resolve();
          }
          else if (op === 'advance') {
            const until = clock + args[0];
            for (;;) {
              const due = [...intervals.values()].filter(t => t.at <= until).sort((a, b) => a.at - b.at)[0];
              if (!due) break;
              clock = due.at; due.at += due.ms; due.run();
            }
            clock = until;
          }
          else if (op === 'unmount') root.unmount();
          else throw new Error('Unknown operation ' + op);
        });
      },
    };
  ` },
  plugins: [{ name: 'controlled-lock-transport', setup(api) {
    api.onResolve({ filter: /^@\/api$/ }, () => ({ path: 'lock-api', namespace: 'fixture' }));
    api.onLoad({ filter: /.*/, namespace: 'fixture' }, () => ({ contents: 'export const fetchLock = () => window.__lockTransport.fetchLock(); export const unlock = pin => window.__lockTransport.unlock(pin);' }));
    api.onResolve({ filter: /^@\/lib\/other-windows$/ }, () => ({ path: fileURLToPath(new URL('../src/lib/other-windows.ts', import.meta.url)) }));
  } }],
});
const ON = { on: true, has_pin: true, timeout_min: 0, timeouts: [0, 1, 5], pin_min: 4, system_door: true };
const OFF = { ...ON, on: false };
const cases = [];
const check = (name, run, strict = false) => cases.push({ name, run, strict });
check('initial enabled/disabled answers keep launch semantics', async f => {
  assert.deepEqual(await f.state(), { facts: null, settled: false, locked: true });
  await f.run('read', 0, OFF); assert.equal((await f.state()).locked, false);
  await f.run('reread'); await f.run('read', 1, ON);
  assert.equal((await f.state()).locked, false, 'enabling is configuration, not a lock-now action');
});
check('first unknown read failure retains the existing fallback and can recover', async f => {
  await f.run('failRead', 0); assert.deepEqual(await f.state(), { facts: null, settled: true, locked: false });
  await f.run('reread'); await f.run('read', 1, ON);
  assert.deepEqual(await f.state(), { facts: ON, settled: true, locked: true });
});
check('failed refresh cannot unlock an already locked face', async f => {
  await f.run('read', 0, ON); await f.run('reread'); await f.run('failRead', 1);
  assert.deepEqual(await f.state(), { facts: ON, settled: true, locked: true });
});
check('LOCK_TOLD failure keeps this window locked', async f => {
  await f.run('read', 0, ON); await f.run('notify'); await f.run('failRead', 1);
  assert.equal((await f.state()).locked, true);
});
check('successful PIN stays unlocked through a failed refresh', async f => {
  await f.run('read', 0, ON); await f.run('open'); await f.run('pin', 0, true);
  assert.equal((await f.all()).outcomes[0].value, true);
  await f.run('reread'); await f.run('failRead', 1); assert.equal((await f.state()).locked, false);
  await f.run('now'); assert.equal((await f.state()).locked, true);
});
check('system-authenticated opening stays unlocked through a failed refresh', async f => {
  await f.run('read', 0, ON); await f.run('opened'); await f.run('notify'); await f.run('failRead', 1);
  assert.equal((await f.state()).locked, false);
});
check('wrong PIN and verification transport failure do not unlock', async f => {
  await f.run('read', 0, ON); await f.run('open'); await f.run('pin', 0, false);
  assert.equal((await f.all()).outcomes[0].value, false); assert.equal((await f.state()).locked, true);
  await f.run('open'); await f.run('failPin', 1);
  assert.match((await f.all()).outcomes[1].error, /verification unavailable/); assert.equal((await f.state()).locked, true);
});
check('new settings do not lock or unlock an initialized face', async f => {
  await f.run('read', 0, ON); await f.run('notify'); await f.run('read', 1, OFF);
  assert.equal((await f.state()).locked, true);
  await f.run('open'); await f.run('pin', 0, true);
  await f.run('notify'); await f.run('read', 2, ON); assert.equal((await f.state()).locked, false);
});
check('idle lock survives a later configuration read failure', async f => {
  await f.run('read', 0, { ...ON, timeout_min: 1 }); await f.run('opened');
  await f.run('advance', 60_000); assert.equal((await f.state()).locked, true);
  await f.run('reread'); await f.run('failRead', 1); assert.equal((await f.state()).locked, true);
});
check('manual lock during refresh is not cleared by its failure', async f => {
  await f.run('read', 0, ON); await f.run('opened'); await f.run('reread'); await f.run('now'); await f.run('failRead', 1);
  assert.equal((await f.state()).locked, true);
});
check('overlapping reads keep the latest successful configuration', async f => {
  await f.run('read', 0, ON); await f.run('reread'); await f.run('reread');
  await f.run('read', 2, { ...ON, timeout_min: 5 }); await f.run('read', 1, OFF);
  assert.equal((await f.state()).facts.timeout_min, 5); assert.equal((await f.state()).facts.on, true);
});
check('requesting a new read immediately revokes an old initial failure', async f => {
  await f.run('supersede', 0, null);
  assert.deepEqual(await f.state(), { facts: null, settled: false, locked: true });
  await f.run('read', 1, ON); assert.equal((await f.state()).locked, true);
});
check('requesting a new read immediately revokes an old initial disabled answer', async f => {
  await f.run('supersede', 0, OFF);
  assert.deepEqual(await f.state(), { facts: null, settled: false, locked: true });
  await f.run('read', 1, ON); assert.equal((await f.state()).locked, true);
});
check('old read failure after a newer successful read has no effect', async f => {
  await f.run('read', 0, ON); await f.run('reread'); await f.run('notify');
  await f.run('read', 2, { ...ON, timeout_min: 5 }); await f.run('failRead', 1);
  assert.equal((await f.state()).locked, true); assert.equal((await f.state()).facts.timeout_min, 5);
});
check('unknown storage keys do not refresh the lock', async f => {
  await f.run('read', 0, ON); await f.run('notify', 'another-setting'); assert.equal((await f.all()).reads, 1);
});
check('unmount removes store subscription, idle timer and old result ownership', async f => {
  await f.run('read', 0, { ...ON, timeout_min: 1 }); await f.run('reread'); await f.run('unmount');
  const before = await f.all(); assert.equal(before.intervals, 0);
  await f.run('notify'); await f.run('failRead', 1); await f.run('advance', 120_000);
  const after = await f.all(); assert.equal(after.reads, before.reads); assert.equal(after.history.length, before.history.length);
});
check('StrictMode cleanup ignores the abandoned first mount read', async f => {
  assert.equal((await f.all()).reads, 2); await f.run('read', 1, ON); await f.run('failRead', 0);
  assert.deepEqual(await f.state(), { facts: ON, settled: true, locked: true });
}, true);

const browser = await chromium.launch({ channel: 'chrome', headless: true });
let failed = 0;
console.log(`use-lock SHA-256: ${beforeHash}; React browser: ${browser.version()}`);
try {
  for (const test of cases) {
    const context = await browser.newContext();
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
      await page.setContent('<div id="root"></div>');
      await page.addScriptTag({ content: bundle.outputFiles[0].text });
      await page.evaluate(strict => window.__lockTest.mount(strict), test.strict);
      const all = () => page.evaluate(() => window.__lockTest.read());
      const run = (op, ...args) => page.evaluate(({ op, args }) => window.__lockTest.run(op, args), { op, args });
      const state = async () => { const { facts, locked, settled } = await all(); return { facts, settled, locked }; };
      await test.run({ run, all, state });
      assert.deepEqual(errors, [], 'No uncaught browser errors');
      console.log('PASS', test.name);
    } catch (error) { failed++; console.error('FAIL', test.name, '\n ', error.message); }
    finally { await context.close(); }
  }
} finally { await browser.close(); }
assert.equal(fingerprint(), beforeHash, 'The tested production source must not change during the run');
console.log(`lock-state: ${cases.length - failed}/${cases.length} passed`);
if (failed) process.exitCode = 1;
