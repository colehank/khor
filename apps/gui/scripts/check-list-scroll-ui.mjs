import assert from 'node:assert/strict';
import {mkdtempSync, writeFileSync, rmSync, mkdirSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {basename, join} from 'node:path';
import {tmpdir} from 'node:os';
import {createServer} from 'vite';
import {chromium, webkit} from 'playwright-core';

const gui = fileURLToPath(new URL('..', import.meta.url));
const fixture = mkdtempSync(join(gui, '.list-scroll-check-'));
const out = join(process.env.RUNNER_TEMP || tmpdir(), 'khor-list-scroll-evidence');
mkdirSync(out, {recursive: true});
writeFileSync(join(fixture, 'index.html'), '<html data-narrow data-theme="dark"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="icon" href="data:,"><div id="root"></div><script type="module" src="./probe.tsx"></script></html>');
writeFileSync(join(fixture, 'probe.tsx'), `
import React from 'react';import {createRoot} from 'react-dom/client';
import {ListPane} from '/src/components/ListPane';import {SideList} from '/src/components/SideList';
import {TooltipProvider} from '/src/components/ui/tooltip';import {band,followScrollEdge} from '/src/lib/scroll-edge';
import '/src/styles/tokens.css';import '/src/styles/app.css';
window.refreshes=0;window.actions=[];followScrollEdge();
const rows=Array.from({length:50},(_,i)=>({id:'session-'+i,group:i<8?'pin':'',pinned:i<8,face:{art:<span>k</span>},title:'Session '+i,sub:'Latest message '+i,find:'Session '+i,
menu:[{key:'pin',label:'置顶',run:()=>window.actions.push('pin')},{key:'delete',label:'删除',destructive:true,run:()=>window.actions.push('delete')}],onOpen:()=>{}}));
createRoot(document.getElementById('root')).render(<TooltipProvider><div data-nav-host data-nav-shown className="relative flex h-screen flex-col overflow-hidden">
<div ref={band} data-band data-side-l0 className="flex h-20 flex-none items-center px-page text-title">会话</div>
<div className="relative flex min-h-0 flex-1 flex-col overflow-hidden"><ListPane mark="data-list" name="Sessions" pane={null} narrow onRefresh={async()=>{window.refreshes++;}}><SideList rows={rows} label={group=>group==='pin'?'置顶':'最近'}/></ListPane></div>
</div></TooltipProvider>);
`);

let server, browser;
const results = [];
try {
  server = await createServer({configFile: join(gui, 'vite.config.ts'), root: gui, cacheDir: join(fixture, '.vite'), optimizeDeps: {entries: [join(fixture, 'index.html')]}, server: {host: '127.0.0.1', port: 23839, strictPort: true, hmr: false}, logLevel: 'error'});
  await server.listen();
  const port = server.httpServer.address().port;
  for (const [name, engine] of [['chromium', chromium], ...(process.env.QUALITY_ENGINES?.includes('webkit') ? [['webkit', webkit]] : [])]) {
    browser = await engine.launch({headless: true, ...(name === 'chromium' && process.env.KHOR_TEST_CHROME ? {channel: 'chrome'} : {})});
    const page = await browser.newPage({viewport: {width: 390, height: 844}, isMobile: true, hasTouch: true});
    page.setDefaultTimeout(12000);
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${port}/${basename(fixture)}/index.html`);
    await page.locator('[data-side-row="session-49"]').waitFor({state: 'attached'});
    const portBox = page.locator('[data-pane-body]');
    const geometry = () => portBox.evaluate(el => ({height: el.clientHeight, content: el.scrollHeight, owner: el.hasAttribute('data-band-scroll')}));
    const before = await geometry();
    assert(before.content > before.height * 2, 'the fixture must have a scrollable list');
    assert(before.owner, 'the fixed heading must own the list scroll port');
    const touch = (type, x, y, cancelable = true, target = '[data-pane-body]', count = 1) => page.evaluate(({type, x, y, cancelable, target, count}) => {
      const el = document.querySelector(target);
      const point = new Touch({identifier: 1, target: el, clientX: x, clientY: y});
      const points = type === 'touchend' || type === 'touchcancel' ? [] : Array.from({length: count}, (_, i) => i === 0 ? point : new Touch({identifier: i + 1, target: el, clientX: x + 20, clientY: y}));
      const event = new TouchEvent(type, {bubbles: true, cancelable, touches: points, targetTouches: points, changedTouches: [point]});
      el.dispatchEvent(event);
      return event.defaultPrevented;
    }, {type, x, y, cancelable, target, count});
    const frame = () => page.evaluate(() => new Promise(requestAnimationFrame));

    await touch('touchstart', 200, 400);
    await touch('touchmove', 200, 330);
    await portBox.evaluate(el => {el.scrollTop = 80;});
    await touch('touchmove', 200, 430, false); await frame();
    assert.equal(await page.locator('[data-pull-host]').count(), 0, 'reversing an established native scroll must not transform its scroll port');
    assert.deepEqual(await geometry(), before, 'continuous scrolling must keep one port and stable content geometry');
    await touch('touchcancel', 200, 430);
    assert.equal(await page.evaluate(() => window.refreshes), 0);

    await portBox.evaluate(el => {el.scrollTop = 0;}); await frame();
    await touch('touchstart', 200, 200);
    assert.equal(await touch('touchmove', 200, 360), true);
    await touch('touchcancel', 200, 360); await frame();
    assert.equal(await page.evaluate(() => window.refreshes), 0, 'OS gesture cancellation must not start a refresh');
    await page.waitForFunction(() => !document.querySelector('[data-pull-host]'));
    await touch('touchstart', 200, 200); await touch('touchmove', 200, 360);
    await touch('touchstart', 200, 360, true, '[data-pane-body]', 2); await frame();
    assert.equal(await page.locator('[data-pull-host]').count(), 0, 'a second finger must release the pull transform');
    await touch('touchcancel', 200, 360);

    const row = '[data-side-row="session-1"] [data-side-open]';
    await touch('touchstart', 300, 230, true, row);
    await touch('touchmove', 150, 233, true, row);
    await touch('touchend', 150, 233, true, row); await frame();
    assert.equal(await page.locator('[data-row-revealed]').count(), 1, 'horizontal list actions must still open');
    assert.equal(await page.locator('[data-pull-host]').count(), 0);
    assert.equal(await page.locator('[data-row-revealed] [data-row-action="pin"]').count(), 1);
    assert.equal(await page.locator('[data-row-revealed] [data-row-action="delete"]').count(), 1);

    await touch('touchstart', 200, 200);
    await touch('touchmove', 200, 360);
    await touch('touchend', 200, 360);
    await page.waitForFunction(() => window.refreshes === 1 && !document.querySelector('[data-pull-host]'));
    assert.deepEqual(await geometry(), before);
    assert.deepEqual(errors, []);
    results.push({engine: name, syntheticTouchBoundary: true, geometry: before, realIphoneGesture: 'pending'});
    await browser.close(); browser = null;
  }
  console.log('List scroll UI: native ownership, direction reversal, cancellation, refresh and horizontal actions passed');
} catch (error) {
  if (browser) {try {await browser.contexts()[0]?.pages()[0]?.screenshot({path: join(out, 'failure.png')});} catch {}}
  throw error;
} finally {
  writeFileSync(join(out, 'report.json'), JSON.stringify(results, null, 2));
  await browser?.close(); await server?.close(); rmSync(fixture, {recursive: true, force: true});
}
