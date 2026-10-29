import {installAttachmentLeases} from "./fixtures/attachment-leases.mjs";
// Actual ChatView/React and production api.ts; only Tauri invoke is a fixture.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync, existsSync} from 'node:fs';
import {basename, join} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium, webkit} from 'playwright-core';

import {chatReplayCases, exerciseChatReplay} from './fixtures/chat-replay-cases.mjs';
const known = chatReplayCases;
const scenarios = process.env.CHAT_REPLAY_SCENARIOS?.split(',').filter(Boolean) ?? known;
assert(scenarios.length > 0, 'Select at least one chat replay scenario');
assert.equal(new Set(scenarios).size, scenarios.length, 'Chat replay scenarios must not repeat');
for (const scenario of scenarios) assert(known.includes(scenario), `Unknown chat replay scenario: ${scenario}`);
const gui = fileURLToPath(new URL('..', import.meta.url));
assert(existsSync(join(gui, 'src/gen/catalog.ts')), 'Generate the catalog/bindings before running the component fixture');
const evidence = process.env.CHAT_REPLAY_CHECK_DIR || mkdtempSync(join(tmpdir(), 'khor-chat-replay-'));
mkdirSync(evidence, {recursive: true});
const sourceFiles = ['src/lib/attachment-view.ts', 'src/gen/bindings/AttachmentKind.ts', 'scripts/fixtures/attachment-leases.mjs', 'src/views/ChatView.tsx', 'scripts/check-chat-replay.mjs', 'scripts/fixtures/chat-replay-cases.mjs', 'src/api.ts', 'src/lib/acp-view.ts', 'src/gen/bindings/ChatFrame.ts', 'src/gen/catalog.ts'];
const fingerprint = () => Object.fromEntries(sourceFiles.map(file => [file, createHash('sha256').update(readFileSync(join(gui, file))).digest('hex')]));
const sources = fingerprint(), results = [];
const fixture = mkdtempSync(join(gui, '.chat-replay-check-'));
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
const until = async (check, what) => {
  const end = Date.now() + 10000;
  while (!await check()) { assert(Date.now() < end, what); await wait(20); }
};
writeFileSync(join(fixture, 'index.html'), '<link rel="icon" href="data:,"><div id="root"></div><script type="module" src="./probe.tsx"></script>');
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
    const errors = [], consoleErrors = [], networkErrors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => { if (message.type() === 'error') consoleErrors.push(message.text()); });
    page.on('response', response => { if (response.status() >= 400) networkErrors.push({url: response.url(), status: response.status()}); });
    await page.addInitScript(installAttachmentLeases);
    await page.addInitScript(({scenario}) => {
      const empty = {frames: [], next: 0, gone: false, lost: false, sending: 0, sending_of: 0, stall: null};
      const wire = window.__chatWire = {calls: [], pending: [],
        reply: (i, change = {}) => wire.pending[i].resolve({...empty, ...change}),
      };
      window.__TAURI_INTERNALS__ = {
        metadata: {currentWindow: {label: 'main'}, currentWebview: {label: 'main', windowLabel: 'main'}},
        transformCallback: () => 0, unregisterCallback: () => {}, convertFileSrc: path => path,
        invoke: async (cmd, args = {}) => {
          const call = {cmd, args, at: Date.now()}; wire.calls.push(call);
          const admission = window.__attachmentLeases(cmd,args);
          if (admission.handled) return admission.value;
          if (cmd === 'chat_wait' || cmd === 'chat_poll') {
            return new Promise((resolve, reject) => wire.pending.push({resolve, reject, call}));
          }
          if (cmd === 'chat_open') return true;
          if (cmd === 'devices' || cmd === 'workspaces') return [];
          if (cmd === 'plugin:event|listen') return 0;
          return null;
        },
      };
      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {unregisterListener: () => {}};
    }, {scenario});
    const polls = () => page.evaluate(() => window.__chatWire.calls.filter(call => call.cmd === 'chat_wait' || call.cmd === 'chat_poll'));
    try {
      await page.clock.install({time: new Date('2026-10-25T00:00:00Z')});
      await page.clock.pauseAt(new Date('2026-10-25T00:00:00Z'));
      await page.goto(`http://127.0.0.1:${server.config.server.port}/${basename(fixture)}/index.html`);
      await until(async () => (await polls()).length > 0, 'actual ChatView started a poll');
      await exerciseChatReplay(scenario, page, polls);
      assert.deepEqual(errors, []);
      assert.deepEqual(consoleErrors, []);
      results.push({engine: name, scenario, ok: true, polls: await polls(), errors, consoleErrors, networkErrors}); console.log('PASS', name, scenario);
    } catch (error) {
      results.push({engine: name, scenario, ok: false, error: error.message, polls: await polls().catch(() => []), errors, consoleErrors, networkErrors});
      console.error('FAIL', name, scenario, error.message);
      await page.screenshot({path: join(evidence, `${name}-${scenario}.png`)}).catch(() => {});
    } finally { await page.close(); }
  }
  await browser.close(); browser = null;
}
try {
  process.env.KHOR_FACE_GATE = '1';
  server = await createServer({root: gui, cacheDir: join(fixture, '.vite'), optimizeDeps: {entries: [join(fixture, 'index.html')]}, server: {host: '127.0.0.1', port: Number(process.env.CHAT_REPLAY_PORT || 23827), strictPort: true, hmr: false}, clearScreen: false, logLevel: 'error'});
  await server.listen();
  await run(chromium, 'chrome');
  if (process.env.CHAT_REPLAY_WEBKIT === '1') await run(webkit, 'webkit');
  const sourcesAfter = fingerprint();
  assert.deepEqual(sourcesAfter, sources, 'source stayed fixed during the run');
  writeFileSync(join(evidence, 'report.json'), JSON.stringify({sources, sourcesAfter, results, finished: true}, null, 2));
  assert(results.every(result => result.ok), `${results.filter(result => !result.ok).length} chat replay regressions failed`);
  console.log(`chat replay: ${results.length}/${results.length} passed`, evidence);
} finally { await browser?.close(); await server?.close(); rmSync(fixture, {recursive: true, force: true}); }
