import {installAttachmentLeases} from "./fixtures/attachment-leases.mjs";
// Actual DetailPane/RecoveryActions/IDB plus original ChatView/Reader; only
// external Tauri IO is controlled. Relationship/action cases remain separate from history cases.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync, existsSync} from 'node:fs';
import {basename, join} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium, webkit} from 'playwright-core';

import {recoveryActionCases, recoveryProbe, installRecoveryActionsTransport} from './fixtures/recovery-actions-ui.mjs';
import {chatRecoveryCases, exerciseChatRecovery, installRecoveryTransport} from './fixtures/chat-recovery-cases.mjs';
const known = [...chatRecoveryCases,...recoveryActionCases];
const scenarios = process.env.CHAT_RECOVERY_SCENARIOS?.split(',').filter(Boolean) ?? known;
assert(scenarios.length > 0, 'Select at least one chat recovery scenario');
assert.equal(new Set(scenarios).size, scenarios.length, 'Chat recovery scenarios must not repeat');
for (const scenario of scenarios) assert(known.includes(scenario), `Unknown chat recovery scenario: ${scenario}`);
const gui = fileURLToPath(new URL('..', import.meta.url));
assert(existsSync(join(gui, 'src/gen/catalog.ts')), 'Generate the catalog/bindings before running the component fixture');
const evidence = process.env.CHAT_RECOVERY_CHECK_DIR || mkdtempSync(join(tmpdir(), 'khor-chat-recovery-'));
mkdirSync(evidence, {recursive: true});
const sourceFiles = ['src/browse/TabStrip.tsx', 'src/components/PillNotice.tsx', 'src/lib/pill-notices.ts', 'src/lib/recovery-relationship.ts', 'src/styles/app.css', 'src/lib/attachment-view.ts', 'src/gen/bindings/AttachmentKind.ts', 'scripts/fixtures/attachment-leases.mjs', 'src/views/ChatView.tsx', 'src/views/DetailPane.tsx', 'src/views/RecoveryActions.tsx', 'src/views/TerminalPane.tsx', 'src/views/SessionPreferences.tsx', 'src/hooks/use-session-recovery.ts', 'src/lib/session-recovery-intent.ts', 'scripts/fixtures/recovery-actions-ui.mjs', 'scripts/check-chat-recovery.mjs', 'scripts/fixtures/chat-recovery-cases.mjs', 'src/api.ts', 'src/lib/acp-view.ts', 'src/gen/bindings/ChatFrame.ts', 'src/gen/bindings/SessionRow.ts', 'src/gen/bindings/SessionRecovery.ts', 'src/gen/bindings/RecoveryResult.ts', 'src/gen/bindings/ShellRunResult.ts', 'src/gen/bindings/RecoveryToken.ts', 'src/gen/catalog.ts'];
const fingerprint = () => Object.fromEntries(sourceFiles.map(file => [file, createHash('sha256').update(readFileSync(join(gui, file))).digest('hex')]));
const catalog=readFileSync(join(gui,'src/gen/catalog.ts'),'utf8');for(const word of ['recovery_title','recovery_join','recovery_choose_directory','recovery_waiting_owner'])assert(catalog.includes(word),'Run the actual current catalog producer before this fixture: '+word);
const sources = fingerprint(), results = [];
const planned={legacy:scenarios.filter(name=>chatRecoveryCases.includes(name)),s3:scenarios.filter(name=>recoveryActionCases.includes(name))};
const fixture = mkdtempSync(join(gui, '.chat-recovery-check-'));
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
const until = async (check, what) => {
  const end = Date.now() + 10000;
  while (!await check()) { assert(Date.now() < end, what); await wait(20); }
};
writeFileSync(join(fixture, 'index.html'), '<link rel="icon" href="data:,"><div id="root"></div><script type="module" src="./probe.tsx"></script>');
writeFileSync(join(fixture, 'probe.tsx'), recoveryProbe);
let server, browser;
async function run(engine, name) {
  browser = await engine.launch(name === 'chrome' ? {channel: (process.env.PLAYWRIGHT_CHROMIUM_CHANNEL ?? 'chrome') || undefined, headless: true} : {headless: true});
  for (const scenario of scenarios) {
    const context = await browser.newContext({viewport:{width:1100,height:760}});
    const page = await context.newPage();
    page.setDefaultTimeout(12000); page.setDefaultNavigationTimeout(60000);
    const errors = [], consoleErrors = [], networkErrors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => { if (message.type() === 'error') consoleErrors.push(message.text()); });
    page.on('response', response => { if (response.status() >= 400) networkErrors.push({url: response.url(), status: response.status()}); });
    await page.addInitScript(installAttachmentLeases);
    await page.addInitScript(installRecoveryTransport);
    await page.addInitScript(installRecoveryActionsTransport);
    const polls = () => page.evaluate(() => window.__chatWire.calls.filter(call => call.cmd === 'chat_wait' || call.cmd === 'chat_poll'));
    const facts=()=>page.evaluate(()=>({origin:location.origin,calls:window.__recoveryWire?.calls.slice(-64),peers:window.__recoveryWire?.peers,terminals:window.__recoveryWire?.terminals,heldOffers:window.__recoveryWire?.offers.map(x=>({id:x.args.id,done:x.done})),heldActions:window.__recoveryWire?.actions.map(x=>({cmd:x.cmd,args:x.args,done:x.done})),heldDrops:window.__recoveryWire?.drops.map(x=>({lease:x.lease,done:x.done}))})).catch(()=>null);
    try {
      await page.clock.install({time: new Date('2026-10-25T00:00:00Z')});
      await page.clock.pauseAt(new Date('2026-10-25T00:00:00Z'));
      await page.goto(`http://127.0.0.1:${server.config.server.port}/${basename(fixture)}/index.html?recovery_case=${encodeURIComponent(scenario)}`);
      if(scenario==='automatic-entry-and-later-exit') await until(()=>page.evaluate(()=>window.__recoveryWire.actions.length===1),'entry continuation reached controlled IO');
      else await until(async () => (await polls()).length > 0, 'actual ChatView started a poll');
      await exerciseChatRecovery(scenario, page, polls, async peer=>{
        peer.setDefaultTimeout(12000);peer.setDefaultNavigationTimeout(60000);
        peer.on('pageerror',error=>errors.push('peer: '+error.message));
        peer.on('console',message=>{if(message.type()==='error')consoleErrors.push('peer: '+message.text());});
        await peer.addInitScript(installAttachmentLeases);await peer.addInitScript(installRecoveryTransport);await peer.addInitScript(installRecoveryActionsTransport);
        await peer.clock.install({time:new Date('2026-10-25T00:00:00Z')});await peer.clock.pauseAt(new Date('2026-10-25T00:00:00Z'));
      });
      assert.deepEqual(errors, []);
      assert.deepEqual(consoleErrors, []);
      results.push({engine: name, scenario, ok: true, facts:await facts(), polls: await polls(), errors, consoleErrors, networkErrors}); console.log('PASS', name, scenario);
    } catch (error) {
      results.push({engine: name, scenario, ok: false, error: error.message, facts:await facts(), polls: await polls().catch(() => []), errors, consoleErrors, networkErrors});
      console.error('FAIL', name, scenario, error.message);
      await page.screenshot({path: join(evidence, `${name}-${scenario}.png`)}).catch(() => {});
    } finally { await context.close(); }
  }
  await browser.close(); browser = null;
}
try {
  process.env.KHOR_FACE_GATE = '1';
  server = await createServer({configFile: join(gui, 'vite.config.ts'), root: gui, cacheDir: join(fixture, '.vite'), optimizeDeps: {entries: [join(fixture, 'index.html')]}, server: {host: '127.0.0.1', port: Number(process.env.CHAT_RECOVERY_PORT || 23827), strictPort: true, hmr: false}, clearScreen: false, logLevel: 'error'});
  await server.listen();
  await run(chromium, 'chrome');
  if (process.env.CHAT_RECOVERY_WEBKIT === '1') await run(webkit, 'webkit');
  const sourcesAfter = fingerprint();
  assert.deepEqual(sourcesAfter, sources, 'source stayed fixed during the run');
  writeFileSync(join(evidence, 'report.json'), JSON.stringify({sources, sourcesAfter, planned, results, finished: true}, null, 2));
  assert(results.every(result => result.ok), `${results.filter(result => !result.ok).length} chat recovery regressions failed`);
  console.log(`chat recovery: ${results.length}/${results.length} passed`, evidence);
} finally { await browser?.close(); await server?.close(); rmSync(fixture, {recursive: true, force: true}); }
