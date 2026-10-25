// Real ChatView + production API; IPC replies and selected files are synthetic.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync, existsSync} from 'node:fs';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium, webkit} from 'playwright-core';

import {installChatCancelWire} from './fixtures/chat-cancel-wire.mjs';
import {extraCancelCases, exerciseCancellation} from './fixtures/chat-cancel-cases.mjs';
const known = ['stop-error', 'restore-empty', 'legacy-reconnect', 'old-receipt', 'legacy-receipt-first', ...extraCancelCases];
const scenarios = process.env.CHAT_CANCEL_SCENARIOS?.split(',').filter(Boolean) ?? known;
assert(scenarios.length && new Set(scenarios).size === scenarios.length, 'Select nonempty, distinct cancellation scenarios');
for (const scenario of scenarios) assert(known.includes(scenario), `Unknown cancellation scenario: ${scenario}`);
const gui = fileURLToPath(new URL('..', import.meta.url));
assert(existsSync(join(gui, 'src/gen/catalog.ts')), 'Generate same-source catalog/bindings first');
const fixture = mkdtempSync(join(gui, '.chat-cancel-check-'));
const evidence = process.env.CHAT_CANCEL_CHECK_DIR || mkdtempSync(join(tmpdir(), 'khor-chat-cancel-'));
mkdirSync(evidence, {recursive: true});
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
const until = async (check, what) => {
  const end = Date.now() + 8000;
  while (!await check()) { assert(Date.now() < end, what); await wait(20); }
};
const files = ['src/views/ChatView.tsx', 'src/views/chat-files.tsx', 'src/api.ts', 'src/App.tsx', 'src/lib/chat-drafts.ts', 'src/gen/bindings/ChatFrame.ts', '../../crates/catalog/zh.toml', '../../crates/catalog/en.toml', 'scripts/check-chat-cancel.mjs', 'scripts/fixtures/chat-cancel-cases.mjs', 'scripts/fixtures/chat-cancel-wire.mjs', 'scripts/quality-preview.vite.ts', 'scripts/session-navigation-preview.vite.ts', 'scripts/ai-elements-preview.vite.ts'];
const fingerprint = () => Object.fromEntries(files.map(file => [file, createHash('sha256').update(readFileSync(join(gui, file))).digest('hex')]));
const sources = fingerprint(), results = [];
writeFileSync(join(fixture, 'index.html'), '<div id="root"></div><script type="module" src="./probe.tsx"></script>');
writeFileSync(join(fixture, 'probe.tsx'), `
import React,{useState} from 'react'; import {createRoot} from 'react-dom/client';
import {ChatView} from '/src/views/ChatView'; import {TooltipProvider} from '/src/components/ui/tooltip';
import {handOff} from '/src/lib/composer-handoff';
import {retainCancelledChatDrafts} from '/src/lib/chat-drafts';
import {noticesOf} from '/src/lib/pill-notices';
import {tabAct} from '/src/lib/tab-acts';
retainCancelledChatDrafts(['alpha','beta']);
import '/src/styles/tokens.css'; import '/src/styles/app.css';
document.documentElement.dataset.theme='light'; handOff('terminal.txt','synthetic terminal block');
function Probe(){const [id,setId]=useState('alpha');window.__cancelReact={switch:()=>setId('beta'),back:()=>setId('alpha'),reload:()=>tabAct(id)?.run(),retry:()=>noticesOf(id).flatMap(n=>n.acts).find(a=>a.key==='retry')?.run()};return <TooltipProvider><div style={{height:'100vh',display:'flex',flexDirection:'column'}}><ChatView key={id} id={id}/></div></TooltipProvider>;}
createRoot(document.getElementById('root')).render(<Probe/>);
`);
let browser, server;
async function run(engine, name) {
  browser = await engine.launch(name === 'chrome' ? {channel: (process.env.PLAYWRIGHT_CHROMIUM_CHANNEL ?? 'chrome') || undefined, headless: true} : {headless: true});
  for (const scenario of scenarios) {
    const page = await browser.newPage({viewport: {width: 1100, height: 800}});
    page.setDefaultTimeout(8000); page.setDefaultNavigationTimeout(60000);
    const errors = [], consoleErrors = []; let layout;
    page.on('pageerror', e => errors.push(e.message));
    page.on('console', message => { if (message.type() === 'error') consoleErrors.push(message.text()); });
    const app = scenario.startsWith('app-');
    const session = app ? 'gui/ai-elements-review' : 'alpha';
    const http = app || scenario === 'web-admission';
    await page.addInitScript(installChatCancelWire, {scenario, http});
    const attach = async selected => {
      const before = await page.locator('[data-chat-pending-file]').count();
      await page.locator('[data-chat-file-input]').setInputFiles(selected);
      await until(async () => await page.locator('[data-chat-pending-file][data-placed]').count() === before + selected.length, 'selected synthetic files landed');
    };
    const emit = async (frames, gone = false, change = {}) => {
      await until(() => page.evaluate(id => window.__cancelWire.polls.some(p => !p.done && p.id === id), session), 'a real poll is waiting for this receipt');
      await page.evaluate(({session, frames, gone, change}) => window.__cancelWire.emit(session, frames, gone, change), {session, frames, gone, change});
    };
    const cancel = async (index, reconnect = false) => {
      await until(() => page.evaluate(index => window.__cancelWire.polls.some(p => !p.done && p.id === window.__cancelWire.sends[index].args.id), index), 'a real poll is waiting for cancellation');
      await page.evaluate(({index, reconnect}) => window.__cancelWire.cancel(index, reconnect), {index, reconnect});
    };
    try {
      const port = server.config.server.port;
      const address = app ? `http://127.0.0.1:${port}/?bridge=${port}&labSession=${session}&labTabs=${session},gui/ai-elements-plan${scenario === 'app-narrow' ? '&phone=1' : ''}` : `http://127.0.0.1:${port}/${fixture.split('/').pop()}/index.html`;
      await page.goto(address);
      await until(() => page.evaluate(id => window.__cancelWire?.polls.some(p => !p.done && p.id === id), session), 'actual ChatView is ready and holding a wait');
      const draft = '  original draft\nwith whitespace  ';
      await page.locator('[data-chat-input]').fill(draft);
      await attach([
        {name: 'tiny.png', mimeType: 'image/png', buffer: Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl2nJ0AAAAASUVORK5CYII=', 'base64')},
        {name: 'notes.txt', mimeType: 'text/plain', buffer: Buffer.from('synthetic file')},
      ]);
      await until(async () => await page.locator('[data-chat-pending-file][data-placed]').count() === 2, 'both synthetic selected files landed');
      await page.locator('[data-chat-send]').click();
      await until(() => page.evaluate(() => window.__cancelWire.sends.length === 1), 'real send reached production API');
      const sent = await page.evaluate(() => window.__cancelWire.sends[0].args);
      assert.equal(sent.images.length, 1); assert.deepEqual(sent.files ?? [], app ? [] : [['terminal.txt', 'synthetic terminal block']]);
      await page.locator('[data-chat-stop]').click();
      if (scenario === 'stop-error') {
        await until(async () => (await page.locator('[role=alert]').allTextContents()).some(text => text.includes('synthetic Stop denied')), 'actual Stop failure is visible');
      } else if (scenario === 'legacy-receipt-first') {
        await emit([{kind: 'send_cancelled', token: sent.token, reconnect: true}], false);
        await page.locator('[data-chat-retry]').waitFor();
        assert((await page.locator('[data-chat-unopened]').innerText()).includes('上传已停止'));
        await page.locator('[data-chat-retry]').click();
        await until(async () => await page.locator('[data-chat-input]').inputValue() === draft, 'retirement receipt is sufficient to reconnect even before batch gone catches up');
      } else if (extraCancelCases.includes(scenario)) {
        const context = {page, draft, sent, session, emit, cancel, until, attach, evidence, engine: name};
        await exerciseCancellation(scenario, context);
        layout = context.layout;
      } else if (scenario === 'old-receipt') {
        assert.match(sent.token ?? '', /^[A-Za-z0-9._:-]{1,128}$/);
        await page.evaluate(() => window.__cancelWire.sends[0].reject(new Error('old completion before receipt')));
        await page.locator('[data-chat-stop]').waitFor({state: 'detached'});
        await page.locator('[data-chat-input]').fill(sent.text);
        await page.locator('[data-chat-send]').click();
        await until(() => page.evaluate(() => window.__cancelWire.sends.length === 2), 'second identical text send entered');
        const second = await page.evaluate(() => window.__cancelWire.sends[1].args);
        assert.equal(second.text, sent.text); assert.notEqual(second.token, sent.token);
        await page.evaluate(() => window.__cancelWire.cancel(0));
        await until(async () => await page.locator('[data-chat-cancelled-draft]').count() === 1 || await page.locator('[data-chat-input]').inputValue() !== '', 'old receipt was processed');
        assert.equal(await page.locator('[data-chat-input]').inputValue(), '', 'an old cancellation must not change the newer turn composer');
        assert.equal(await page.locator('[data-chat-stop]').count(), 1, 'the newer turn remains running');
        assert.equal(await page.locator('[data-chat-cancelled-draft]').count(), 1, 'the old draft remains explicitly recoverable');
        assert.equal(await page.locator('[data-slot=bubble][data-align=end]').count(), 1, 'only the original token bubble is removed');
      } else {
        assert.match(sent.token ?? '', /^[A-Za-z0-9._:-]{1,128}$/, 'every client send has its own correlation token');
        await page.evaluate(reconnect => window.__cancelWire.cancel(0, reconnect), scenario === 'legacy-reconnect');
        if (scenario === 'legacy-reconnect') {
          await page.locator('[data-chat-retry]').waitFor();
          assert((await page.locator('[data-chat-unopened]').innerText()).includes('上传已停止'));
          await page.locator('[data-chat-retry]').click();
        }
        await until(async () => await page.locator('[data-chat-input]').inputValue() === draft, 'cancelled upload restores the exact original input');
        assert.equal(await page.locator('[data-chat-pending-file][data-placed]').count(), 2);
        assert.equal(await page.locator('[data-chat-doc]').count(), 1);
        const reused = await page.evaluate(() => [...document.querySelectorAll('[data-chat-pending-file] img')].some(img => window.__cancelWire.revoked.includes(img.src)));
        assert.equal(reused, false, 'restored thumbnails never reuse revoked object URLs');
      }
      assert.deepEqual(errors, []);
      assert(!consoleErrors.some(message => /Functions are not valid as a React child/.test(message)), 'the recovery caption renders text, not a formatter function');
      results.push({engine: name, scenario, ok: true, sent, layout, consoleErrors}); console.log('PASS', name, scenario);
    } catch (e) {
      results.push({engine: name, scenario, ok: false, error: e.message, errors, consoleErrors, calls: await page.evaluate(() => window.__cancelWire?.calls ?? []).catch(() => [])});
      console.error('FAIL', name, scenario, e.message); await page.screenshot({path: join(evidence, `${name}-${scenario}.png`)}).catch(() => {});
    } finally { await page.close(); }
  }
  await browser.close(); browser = null;
}
try {
  process.env.KHOR_FACE_GATE = '1';
  server = await createServer({configFile: join(gui, 'scripts/quality-preview.vite.ts'), root: gui, cacheDir: join(fixture, '.vite'), optimizeDeps: {entries: [join(fixture, 'index.html')]}, server: {host: '127.0.0.1', port: Number(process.env.CHAT_CANCEL_PORT || 23829), strictPort: true, hmr: false}, clearScreen: false, logLevel: 'error'});
  await server.listen(); await run(chromium, 'chrome');
  if (process.env.CHAT_CANCEL_WEBKIT === '1') await run(webkit, 'webkit');
  assert.deepEqual(fingerprint(), sources, 'source stayed fixed');
  writeFileSync(join(evidence, 'report.json'), JSON.stringify({sources, results, finished: true}, null, 2));
  assert(results.every(r => r.ok), `${results.filter(r => !r.ok).length} cancellation checks failed`);
} finally { await browser?.close(); await server?.close(); rmSync(fixture, {recursive: true, force: true}); }
