// Actual FilePage + CodeMirror + save-road. Only the disk transport is a
// fixture; every write performs content-hash CAS, including overwrite.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {mkdtempSync, writeFileSync, rmSync, mkdirSync, readFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
import {createServer} from 'vite';
import {chromium, webkit} from 'playwright-core';

const knownScenarios = ['mine', 'two-editors', 'newer-external', 'external-aba', 'typing-during-read', 'cancel-during-read', 'discard-during-read', 'close-during-read', 'switch-file-during-read', 'reopen-during-read', 'incomplete-read', 'binary-read', 'take-theirs'];
const scenarios = process.env.EDITOR_OVERWRITE_SCENARIOS?.split(',').filter(Boolean) ?? knownScenarios;
assert(scenarios.length > 0, 'Select at least one overwrite scenario');
assert.equal(new Set(scenarios).size, scenarios.length, 'Overwrite scenarios must not repeat');
for (const scenario of scenarios) assert(knownScenarios.includes(scenario), `Unknown overwrite scenario: ${scenario}`);

const gui = fileURLToPath(new URL('..', import.meta.url));
const fixture = mkdtempSync(join(gui, '.editor-overwrite-check-'));
const evidence = process.env.EDITOR_OVERWRITE_CHECK_DIR || mkdtempSync(join(tmpdir(), 'khor-overwrite-'));
mkdirSync(evidence, {recursive: true});
const hash = text => createHash('sha256').update(text).digest('hex');
const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
const until = async (test, what) => {
  const end = Date.now() + 8000;
  while (!test()) { assert(Date.now() < end, what); await wait(20); }
};
const sourceFiles = ['src/views/FileView.tsx', 'src/lib/save-road.ts', 'scripts/check-editor-overwrite.mjs'];
const fingerprint = () => Object.fromEntries(sourceFiles.map(file => [file, hash(readFileSync(join(gui, file)))]));
const sources = fingerprint(), results = [];
let server, browser;
let stage = 'starting';
const recordStage = value => {
  stage = value;
  writeFileSync(join(evidence, 'report.json'), JSON.stringify({sources, results, stage, finished: false}, null, 2));
  console.log('STAGE', stage);
};
writeFileSync(join(fixture, 'index.html'), '<div id="root"></div><script type="module" src="./probe.tsx"></script>');
writeFileSync(join(fixture, 'probe.tsx'), `
import React,{useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {FilePage} from '/src/views/FilePage';
import {TooltipProvider} from '/src/components/ui/tooltip';
import {discard} from '/src/lib/save-road';
import '/src/styles/tokens.css'; import '/src/styles/app.css';
document.documentElement.dataset.theme='light';
function Editor({name,path='/fixture/note.txt'}) {
 const [drafts,setDrafts]=useState({}); const history=useRef({});
 return <section data-editor={name} style={{position:'relative',height:'100vh',flex:1,minWidth:0}}><FilePage
  machine="overwrite-fixture" path={path} here={false} editorHistory={history.current}
  draft={drafts[path]} onDraft={d=>setDrafts(was=>was[path]===(d||undefined)?was:{...was,[path]:d||undefined})}
  onBack={()=>{}} onGone={()=>{}} onMoved={()=>{}} /></section>;
}
function Probe(){
 const [shown,setShown]=useState(true),[two,setTwo]=useState(false),[path,setPath]=useState('/fixture/note.txt');
 window.__overwrite={second:()=>setTwo(true),discard:()=>discard('overwrite-fixture','/fixture/note.txt'),
  close:()=>setShown(false),reopen:()=>setShown(true),switch:()=>setPath('/fixture/other.txt'),
  cancel:()=>{discard('overwrite-fixture','/fixture/note.txt');setShown(false);}};
 return <TooltipProvider><div style={{display:'flex',height:'100vh'}}>{shown&&<><Editor name="one" path={path}/>{two&&<Editor name="two"/>}</>}</div></TooltipProvider>;
}
createRoot(document.getElementById('root')).render(<Probe/>);
`);

async function run(engine, name) {
  recordStage(`${name} launching browser`);
  browser = await engine.launch(name === 'chrome' ? {channel: (process.env.PLAYWRIGHT_CHROMIUM_CHANNEL ?? 'chrome') || undefined, headless: true} : {headless: true});
  for (const scenario of scenarios) {
    recordStage(`${name} ${scenario} opening page`);
    const page = await browser.newPage({viewport: {width: 1300, height: 760}});
    page.setDefaultTimeout(12000);
    page.setDefaultNavigationTimeout(60000);
    let disk = 'A', other = 'Other file', heldRead = null, holdNextRead = false, mutateAtWrite = null, readKind = 'complete';
    const writes = [], errors = [], consoleErrors = [];
    page.on('console', message => { if (message.type() === 'error') consoleErrors.push(message.text()); });
    page.on('pageerror', error => errors.push(error.message));
    const snapshot = path => {
      const text = path === '/fixture/other.txt' ? other : disk;
      return {text: readKind === 'binary' ? '' : text, digest: readKind === 'complete' ? hash(text) : '',
        total: Buffer.byteLength(text), at_ms: 1, cut: readKind === 'incomplete', binary: readKind === 'binary'};
    };
    await page.route('**/api/*', async route => {
      const cmd = new URL(route.request().url()).pathname.split('/').pop();
      const args = route.request().postDataJSON() || {};
      if (cmd === 'read_text') {
        const read = snapshot(args.path);
        if (holdNextRead) {
          holdNextRead = false;
          await new Promise(resolve => { heldRead = async () => { await route.fulfill({json: read}); resolve(); }; });
          return;
        }
        await route.fulfill({json: read}); return;
      }
      if (cmd === 'write_text') {
        mutateAtWrite?.(); mutateAtWrite = null;
        const current = args.path === '/fixture/other.txt' ? other : disk;
        const accepted = args.was === hash(current);
        writes.push({...args, before: current, accepted});
        if (!accepted) { await route.fulfill({status: 409, json: {error: 'fixture compare-and-write conflict'}}); return; }
        if (args.path === '/fixture/other.txt') other = args.text; else disk = args.text;
        await route.fulfill({json: {digest: hash(args.text), at_ms: 2}}); return;
      }
      const result = cmd === 'stat_text' ? snapshot(args.path) : cmd === 'git_diff' ? {hunks: [], more: false} : cmd === 'git_log' ? {commits: [], more: false} : null;
      await route.fulfill({json: result});
    });
    const pane = who => page.locator(`[data-editor=${who}]`);
    const editor = who => pane(who).locator('.cm-content[contenteditable=true]');
    const type = async (text, who = 'one', save = true) => {
      const modifier = process.platform === 'darwin' ? 'Meta' : 'Control';
      await editor(who).click(); await page.keyboard.press(`${modifier}+a`); await page.keyboard.insertText(text);
      if (save) await page.keyboard.press(`${modifier}+s`);
    };
    const readText = who => pane(who).locator('[data-code-view]').evaluate(node => node.__view.state.doc.toString());
    try {
      recordStage(`${name} ${scenario} navigating`);
      await page.goto(`http://127.0.0.1:${server.config.server.port}/${fixture.split('/').pop()}/index.html`);
      await editor('one').waitFor({timeout: 45000});
      if (scenario === 'two-editors') { await page.evaluate(() => window.__overwrite.second()); await editor('two').waitFor(); }
      await type('B'); await until(() => disk === 'B', 'first edit saves B');
      // This line has A -> B. Another writer restores A; a fresh read of A
      // must not be forwarded through the old line to B.
      disk = 'A'; await type('C');
      await pane('one').locator('[data-save-state=refused]').waitFor();
      const before = writes.length;
      if (scenario === 'take-theirs') {
        await pane('one').locator('[data-take-theirs]').click();
        await page.waitForFunction(() => {
          const node = document.querySelector('[data-editor=one] [data-code-view]');
          return node?.__view?.state.doc.toString() === 'A' && node.querySelector('.cm-content')?.isContentEditable;
        }, undefined, {timeout: 5000});
        assert.equal(writes.length, before, 'taking the disk version must not write the discarded input');
        await type('D'); await until(() => disk === 'D', 'the reloaded file stays editable and saves from its fresh base');
        assert.equal(writes[before].was, hash('A'));
        assert.deepEqual(errors, []);
        results.push({engine: name, scenario, ok: true, disk, writes}); console.log('PASS', name, scenario);
        continue;
      }
      if (scenario.endsWith('-during-read')) holdNextRead = true;
      if (scenario === 'newer-external') mutateAtWrite = () => { disk = 'external-after-read'; };
      if (scenario === 'external-aba') mutateAtWrite = () => { disk = 'external-intermediate'; disk = 'A'; };
      if (scenario === 'incomplete-read') readKind = 'incomplete';
      if (scenario === 'binary-read') readKind = 'binary';
      await pane('one').locator('[data-keep-mine]').click();
      if (holdNextRead || heldRead) {
        await until(() => heldRead !== null, 'overwrite fresh read is in flight');
        if (scenario === 'typing-during-read') await type('C plus typing', 'one', false);
        else if (scenario === 'discard-during-read') await page.evaluate(() => window.__overwrite.discard());
        else if (scenario === 'switch-file-during-read') {
          await page.evaluate(() => window.__overwrite.switch());
          await page.waitForFunction(() => document.querySelector('[data-editor=one] [data-code-view]')?.__view?.state.doc.toString() === 'Other file');
        } else {
          await page.evaluate(action => window.__overwrite[action](), scenario === 'close-during-read' ? 'close' : 'cancel');
          await editor('one').waitFor({state: 'detached'});
          if (scenario === 'reopen-during-read') { await page.evaluate(() => window.__overwrite.reopen()); await editor('one').waitFor(); }
        }
        await heldRead(); heldRead = null;
      }
      if (scenario === 'incomplete-read' || scenario === 'binary-read') {
        await page.waitForFunction(() => document.querySelector('[data-keep-mine]')?.disabled === false);
        await wait(1400); assert.equal(writes.length, before, 'a read without an editable digest cannot start overwrite');
        assert.equal(await readText('one'), 'C', 'a file that became non-editable must not hide the unsaved input');
      } else if (scenario.endsWith('-during-read') && scenario !== 'typing-during-read') {
        await wait(1400); assert.equal(writes.length, before, 'cancelled late read must not start overwrite'); assert.equal(disk, 'A');
        assert.equal(other, 'Other file', 'changing files must not write the other document');
        if (scenario === 'discard-during-read') assert.equal(await readText('one'), 'C', 'discarding a pending write does not erase the mounted editor');
        if (scenario === 'switch-file-during-read') assert.equal(await readText('one'), 'Other file', 'late answer must not replace the new file buffer');
      } else {
        await until(() => writes.length > before, 'overwrite reaches write_text');
        assert.equal(writes[before].was, hash('A'), 'overwrite CAS uses the digest just read, not old line head B');
        if (scenario === 'newer-external') {
          assert.equal(writes[before].accepted, false); assert.equal(disk, 'external-after-read');
          await pane('one').locator('[data-save-state=refused]').waitFor(); assert.equal(await readText('one'), 'C');
        } else {
          await until(() => disk === (scenario === 'typing-during-read' ? 'C plus typing' : 'C'), 'mine and newer typing reach disk');
          await pane('one').locator('[data-save-state=refused]').waitFor({state: 'detached'});
          if (scenario === 'two-editors') {
            await type('other editor', 'two'); await pane('two').locator('[data-save-state=refused]').waitFor();
            assert.equal(disk, 'C', 'independent old buffer still conflicts');
            await pane('two').locator('[data-keep-mine]').click(); await until(() => disk === 'other editor', 'second editor can explicitly overwrite');
            await type('second continues', 'two'); await until(() => disk === 'second continues', 'second editing line continues');
          } else {
            await type('D'); await until(() => disk === 'D', 'fresh line continues with ordinary save');
          }
        }
      }
      assert.deepEqual(errors, []);
      results.push({engine: name, scenario, ok: true, disk, writes}); console.log('PASS', name, scenario);
    } catch (error) {
      const editorState = await page.evaluate(() => {
        const node = document.querySelector('[data-editor=one] [data-code-view]');
        return node ? {text: node.__view?.state.doc.toString(), editable: node.querySelector('.cm-content')?.isContentEditable} : null;
      }).catch(() => null);
      results.push({engine: name, scenario, ok: false, error: error.message, disk, writes, errors, consoleErrors, editorState}); console.error('FAIL', name, scenario, error.message);
      await page.screenshot({path: join(evidence, `${name}-${scenario}.png`)}).catch(() => {});
    } finally {
      recordStage(`${name} ${scenario} closing page`);
      if (heldRead) await heldRead().catch(() => {});
      await page.close();
    }
  }
  recordStage(`${name} closing browser`);
  await browser.close(); browser = null;
  recordStage(`${name} browser closed`);
}
try {
  process.env.KHOR_FACE_GATE = '1';
  // Cache belongs to this temporary fixture even when node_modules is a
  // read-only link to another worktree's dependencies.
  server = await createServer({root: gui, cacheDir: join(fixture, '.vite'), optimizeDeps: {entries: [join(fixture, 'index.html')]}, server: {host: "127.0.0.1", port: Number(process.env.EDITOR_OVERWRITE_PORT || 23827), strictPort: true}, clearScreen: false});
  await server.listen();
  await run(chromium, 'chrome');
  if (process.env.EDITOR_OVERWRITE_WEBKIT === '1') await run(webkit, 'webkit');
  assert.deepEqual(fingerprint(), sources, 'source stayed fixed during the browser run');
  writeFileSync(join(evidence, 'report.json'), JSON.stringify({sources, results, finished: true}, null, 2));
  assert(results.every(result => result.ok), `${results.filter(result => !result.ok).length} overwrite regressions failed`);
  console.log(`editor overwrite: ${results.length}/${results.length} passed`, evidence);
} finally {
  await browser?.close(); await server?.close(); rmSync(fixture, {recursive: true, force: true});
}
