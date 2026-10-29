import {installAttachmentLeases} from "./fixtures/attachment-leases.mjs";
// Actual ChatView/React and production api.ts; only Tauri invoke is a fixture.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync, existsSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium, webkit} from 'playwright-core';

import {chatPollCases, exerciseChatPoll} from './fixtures/chat-poll-cases.mjs';
const known = ['immediate-failure', 'gone-empty', ...chatPollCases];
const scenarios = process.env.CHAT_POLL_SCENARIOS?.split(',').filter(Boolean) ?? known;
assert(scenarios.length > 0, 'Select at least one chat poll scenario');
assert.equal(new Set(scenarios).size, scenarios.length, 'Chat poll scenarios must not repeat');
for (const scenario of scenarios) assert(known.includes(scenario), `Unknown chat poll scenario: ${scenario}`);
const gui = fileURLToPath(new URL('..', import.meta.url));
assert(existsSync(join(gui, 'src/gen/catalog.ts')), 'Generate the catalog/bindings before running the component fixture');
const fixture = mkdtempSync(join(gui, '.chat-poll-check-'));
const evidence = process.env.CHAT_POLL_CHECK_DIR || mkdtempSync(join(tmpdir(), 'khor-chat-poll-'));
mkdirSync(evidence, {recursive: true});
const sourceFiles = ['src/views/ChatView.tsx', 'scripts/check-chat-poll.mjs', 'scripts/fixtures/chat-poll-cases.mjs'];
const fingerprint = () => Object.fromEntries(sourceFiles.map(file => [file, createHash('sha256').update(readFileSync(join(gui, file))).digest('hex')]));
const sources = fingerprint(), results = [];
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
const until = async (check, what) => {
  const end = Date.now() + 10000;
  while (!await check()) { assert(Date.now() < end, what); await wait(20); }
};
writeFileSync(join(fixture, 'index.html'), '<div id="root"></div><script type="module" src="./probe.tsx"></script>');
writeFileSync(join(fixture, 'probe.tsx'), `
import React,{useState} from 'react';
import {createRoot} from 'react-dom/client';
import {ChatView} from '/src/views/ChatView';
import {TooltipProvider} from '/src/components/ui/tooltip';
import {tabAct} from '/src/lib/tab-acts';
import '/src/styles/tokens.css'; import '/src/styles/app.css';
document.documentElement.dataset.theme='light';
function Probe(){
 const [shown,setShown]=useState(true),[id,setId]=useState('alpha');
 window.__chatReact={close:()=>setShown(false),open:()=>setShown(true),switch:()=>setId('beta'),reload:()=>tabAct(id)?.run()};
 return <TooltipProvider><div style={{height:'100vh'}}>{shown&&<ChatView id={id}/>}</div></TooltipProvider>;
}
createRoot(document.getElementById('root')).render(<Probe/>);
`);
let server, browser;
async function run(engine, name) {
  browser = await engine.launch(name === 'chrome' ? {channel: (process.env.PLAYWRIGHT_CHROMIUM_CHANNEL ?? 'chrome') || undefined, headless: true} : {headless: true});
  for (const scenario of scenarios) {
    const page = await browser.newPage({viewport: {width: 1100, height: 760}});
    page.setDefaultTimeout(12000); page.setDefaultNavigationTimeout(60000);
    const errors = [], consoleErrors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => { if (message.type() === 'error') consoleErrors.push(message.text()); });
    await page.addInitScript(installAttachmentLeases);
    await page.addInitScript(({scenario}) => {
      const empty = {frames: [], next: 0, gone: false, lost: false, sending: 0, sending_of: 0, stall: null};
      const wire = window.__chatWire = {mode: ['immediate-failure', 'success-resets', 'backoff-cap', 'retry-attempt', 'http-failure'].includes(scenario) ? 'fail' : ['gone-empty', 'gone-reload'].includes(scenario) ? 'gone' : 'hold', calls: [], pending: [],
        reply: (i, change = {}) => wire.pending[i].resolve({...empty, ...change}),
        reject: i => wire.pending[i].reject(new Error('fixture unavailable')),
        visibility: hidden => { Object.defineProperty(document, 'hidden', {configurable: true, value: hidden}); document.dispatchEvent(new Event('visibilitychange')); },
      };
      window.__TAURI_INTERNALS__ = {
        metadata: {currentWindow: {label: 'main'}, currentWebview: {label: 'main', windowLabel: 'main'}},
        transformCallback: () => 0, unregisterCallback: () => {}, convertFileSrc: path => path,
        invoke: async (cmd, args = {}) => {
          const call = {cmd, args, at: Date.now()}; wire.calls.push(call);
          const admission = window.__attachmentLeases(cmd,args);
          if (admission.handled) return admission.value;
          if (cmd === 'chat_wait' || cmd === 'chat_poll') {
            if (wire.mode === 'fail') throw new Error('fixture unavailable');
            if (wire.mode === 'gone') return {...empty, gone: true};
            return new Promise((resolve, reject) => wire.pending.push({resolve, reject, call}));
          }
          if (cmd === 'chat_open') return false;
          if (cmd === 'devices' || cmd === 'workspaces') return [];
          if (cmd === 'plugin:event|listen') return 0;
          return null;
        },
      };
      if (scenario === 'http-failure') {
        const invoke = window.__TAURI_INTERNALS__.invoke;
        delete window.__TAURI_INTERNALS__;
        const original = window.fetch.bind(window);
        window.fetch = async (input, init = {}) => {
          const url = new URL(typeof input === 'string' ? input : input.url, location.href);
          if (!url.pathname.startsWith('/api/')) return original(input, init);
          const cmd = url.pathname.split('/').pop();
          try {
            const value = await invoke(cmd, JSON.parse(init.body || '{}'));
            return new Response(JSON.stringify(value), {status: 200, headers: {'Content-Type': 'application/json'}});
          } catch {
            return new Response(JSON.stringify('fixture unavailable'), {status: 503, headers: {'Content-Type': 'application/json'}});
          }
        };
      }
      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {unregisterListener: () => {}};
    }, {scenario});
    const polls = () => page.evaluate(() => window.__chatWire.calls.filter(call => call.cmd === 'chat_wait' || call.cmd === 'chat_poll'));
    try {
      await page.clock.install({time: new Date('2026-10-25T00:00:00Z')});
      await page.clock.pauseAt(new Date('2026-10-25T00:00:00Z'));
      await page.goto(`http://127.0.0.1:${server.config.server.port}/${fixture.split('/').pop()}/index.html`);
      await until(async () => (await polls()).length > 0, 'actual ChatView started a poll');
      if (scenario === 'immediate-failure' || scenario === 'gone-empty') {
      await page.clock.runFor(90);
      assert.equal((await polls()).length, 1, scenario === 'immediate-failure' ? 'immediate failure must back off before another request' : 'gone must stop requesting');
      if (scenario === 'immediate-failure') {
        await page.clock.runFor(10);
        assert.equal((await polls()).length, 2, 'first failed retry waits 100ms');
      } else {
        await page.clock.runFor(60_000);
        await page.evaluate(() => window.__chatWire.visibility(true));
        await page.clock.runFor(20);
        await wait(10);
        await page.evaluate(() => window.__chatWire.visibility(false));
        await page.clock.runFor(100);
        assert.equal((await polls()).length, 1, 'gone also rejects pace/visibility kicks');
      }
      } else { await exerciseChatPoll(scenario, page, polls); }
      assert.deepEqual(errors, []);
      results.push({engine: name, scenario, ok: true, polls: await polls()}); console.log('PASS', name, scenario);
    } catch (error) {
      results.push({engine: name, scenario, ok: false, error: error.message, polls: await polls().catch(() => []), errors, consoleErrors});
      console.error('FAIL', name, scenario, error.message);
      await page.screenshot({path: join(evidence, `${name}-${scenario}.png`)}).catch(() => {});
    } finally { await page.close(); }
  }
  await browser.close(); browser = null;
}
try {
  process.env.KHOR_FACE_GATE = '1';
  server = await createServer({root: gui, cacheDir: join(fixture, '.vite'), optimizeDeps: {entries: [join(fixture, 'index.html')]}, server: {host: '127.0.0.1', port: Number(process.env.CHAT_POLL_PORT || 23828), strictPort: true, hmr: false}, clearScreen: false, logLevel: 'error'});
  await server.listen();
  await run(chromium, 'chrome');
  if (process.env.CHAT_POLL_WEBKIT === '1') await run(webkit, 'webkit');
  assert.deepEqual(fingerprint(), sources, 'source stayed fixed during the run');
  writeFileSync(join(evidence, 'report.json'), JSON.stringify({sources, results, finished: true}, null, 2));
  assert(results.every(result => result.ok), `${results.filter(result => !result.ok).length} chat poll regressions failed`);
  console.log(`chat poll: ${results.length}/${results.length} passed`, evidence);
} finally { await browser?.close(); await server?.close(); rmSync(fixture, {recursive: true, force: true}); }
