import {chromium} from 'playwright-core';
import {spawn} from 'node:child_process';
import {mkdir, writeFile} from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
const report = process.env.LAB_REPORT;
await mkdir(report, {recursive: true});
const port = 23918;
const server = spawn('python3', ['-m', 'khor_lab.preview', process.env.LAB_BUNDLE, '--port', String(port)], {
  cwd: path.resolve('../..'), env: {...process.env, PYTHONPATH: 'scripts'}, stdio: 'inherit',
});

async function inspect(browser, scene, size) {
  const context = await browser.newContext({viewport: size, isMobile: size.width < 500, hasTouch: size.width < 500});
  const page = await context.newPage(), errors = [];
  const name = `${scene}-${size.width}x${size.height}`;
  let geometry = null;
  const contentLayout = [];
  async function checkContentToolbar(phase) {
    const kind = phase.startsWith('file') ? 'file' : phase;
    await page.locator(`[data-session-side] [data-content-toolbar="${kind}"]:visible`).waitFor();
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    const layout = await page.evaluate(kind => {
      const head = document.querySelector('[data-session-tabs]')?.closest('[data-region-head]');
      const toolbar = [...document.querySelectorAll(`[data-session-side] [data-content-toolbar="${kind}"]`)].find(el => el.getClientRects().length);
      const box = el => el ? {top:el.getBoundingClientRect().top,bottom:el.getBoundingClientRect().bottom} : null;
      const controls = toolbar ? [...toolbar.querySelectorAll('button')].filter(el => el.getClientRects().length && !el.matches(':disabled,[aria-disabled="true"]')) : [];
      return {head:box(head), toolbar:box(toolbar), controlsReachable:controls.length > 0 && controls.every(el => {
        const rect = el.getBoundingClientRect();
        return el.contains(document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2));
      })};
    }, kind);
    contentLayout.push({phase,...layout});
    assert(layout.head && layout.toolbar, 'The actual session header and content toolbar must be present');
    assert(layout.toolbar.top >= layout.head.bottom - 1, `The content toolbar must stay below the session tabs (${phase}): ${JSON.stringify(layout)}`);
    assert(layout.controlsReachable, `The session tabs must not cover the content controls (${phase})`);
    await page.screenshot({path:path.join(report,`${name}-${phase}.png`)});
  }
  page.on('pageerror', error => errors.push(String(error)));
  page.on('console', message => {
    if (message.type() === 'error' && message.text().includes('[Lab fixture]')) errors.push(message.text().split('\n')[0]);
  });
  try {
    // Select data only; every layout and action belongs to the production App.
    await page.route('**/lab/runtime*', route => route.fulfill({contentType: 'application/json', body: JSON.stringify({mode: 'frontend', scene, tree: 'browser-check', revision: 'one'})}));
    await page.goto(`http://127.0.0.1:${port}/app/?theme=dark`);
    const selector = scene === 'page' ? '[data-content-visible="true"] .mobile-content-page' : scene === 'list' ? '[data-list]' : scene === 'terminal' ? '[data-terminal]'
      : scene === 'content' ? '.cm-editor, [data-file-preview]' : '[data-chat-box]';
    await page.waitForSelector(selector);
    if (scene === 'page') {
      const body = page.locator('[data-content-visible="true"]');
      await page.evaluate(() => {
        let title = false;
        window.addEventListener('pointerdown', event => {
          title = event.target instanceof Element && !!event.target.closest('[data-deck-title]');
        }, {capture: true, once: true});
        window.addEventListener('click', () => {
          window.__labAddressGestureFocus = {title, focused:document.activeElement?.matches('[data-page-address]')};
        }, {once: true});
      });
      await page.screenshot({path:path.join(report,name+'-page.png')});
      await body.locator('[data-deck-title]').first().tap();
      const focus = await page.evaluate(() => window.__labAddressGestureFocus);
      await writeFile(path.join(report,name+'-focus.json'), JSON.stringify(focus,null,2));
      assert.equal(await page.locator('[data-fault]:visible').count(), 0, 'Editing must not render an error boundary');
      assert.equal(focus.title, true, 'The gesture must begin on the actual address title');
      assert.equal(focus.focused, true, 'The address input must focus during the initiating tap, before animation frames');
      await page.screenshot({path:path.join(report,name+'-address.png')});
      await body.locator('[data-content-edit-end]').click();
      const before = await page.evaluate(async () => (await fetch('/api/tabs_list', {method: 'POST', body: JSON.stringify({kind: 'content'})})).json());
      const back = body.locator('[data-content-back]');
      assert.equal(await back.count(), 1, 'A session webpage must expose Back to its content picker');
      await back.click();
      await page.locator('[data-content-picker]:visible').waitFor();
      const after = await page.evaluate(async () => (await fetch('/api/tabs_list', {method: 'POST', body: JSON.stringify({kind: 'content'})})).json());
      assert.deepEqual(after.tabs.map(tab => tab.id), before.tabs.map(tab => tab.id), 'Back must preserve the content tab identities');
      const picker = page.locator('[data-content-picker]:visible');
      if (!(await picker.locator('.mobile-content-browser:not([hidden])').isVisible())) {
        await picker.locator('[data-content-scope]').click();
        await page.locator('[role="menuitem"][data-content-source="browser"]').click();
      }
      await picker.locator(`[data-place="http://127.0.0.1:${port}/lab/example"]`).click();
      await page.locator('[data-content-visible="true"] .mobile-content-page').waitFor();
      await page.locator('[data-content-visible="true"] [data-content-back]').click();
      await picker.locator('[data-content-scope]').click();
      await page.locator('[role="menuitem"][data-content-source="workspace"]').click();
      await picker.locator('[data-file-path="/preview/project/README.md"]').click();
      await page.locator('[data-content-visible="true"] .cm-editor, [data-content-visible="true"] [data-file-preview]').waitFor();
      const final = await page.evaluate(async () => (await fetch('/api/tabs_list', {method: 'POST', body: JSON.stringify({kind: 'content'})})).json());
      assert.deepEqual(final.tabs.map(tab => tab.id), before.tabs.map(tab => tab.id), 'Recent pages and workspace files must reuse the same content tab');
    }
    if (scene === 'content') {
      const document = await page.locator('.cm-editor, [data-file-preview]').first().innerText();
      if (!document.includes('fixture data')) errors.push('Actual file body is missing');
      if (size.width >= 500) {
        await checkContentToolbar('file');
        for (const width of [1024,1280]) {
          await page.setViewportSize({width,height:size.height});
          await checkContentToolbar(`file-${width}`);
        }
        await page.locator('[data-session-side] [data-content-back]:visible').click();
        await checkContentToolbar('picker');
        const picker = page.locator('[data-content-picker]:visible');
        await picker.locator('[data-content-source="browser"]').click();
        await picker.locator(`[data-place="http://127.0.0.1:${port}/lab/example"]`).click();
        await checkContentToolbar('page');
      }
    }
    if (size.width < 500 && ['gui', 'codex'].includes(scene)) await page.waitForSelector('[data-session-compose-row]');
    await page.waitForTimeout(800);
    assert.equal(await page.locator('[data-fault]:visible').count(), 0, 'The production App must render without an error boundary');
    geometry = await page.evaluate(() => {
      const row = document.querySelector('[data-session-compose-row]');
      const avatar = document.querySelector('[data-lab-composer-session-slot]');
      const capsule = document.querySelector('[data-chat-box]');
      const box = el => el ? {top: el.getBoundingClientRect().top, bottom: el.getBoundingClientRect().bottom} : null;
      return {height: innerHeight, row: box(row), avatar: box(avatar), capsule: box(capsule), avatarOutsideCapsule: !!avatar && !capsule?.contains(avatar), overflow: document.documentElement.scrollWidth > innerWidth + 1};
    });
    if (size.width < 500 && ['gui', 'codex'].includes(scene)) {
      if (!geometry.avatarOutsideCapsule) errors.push('Avatar must be the production sibling of the composer capsule');
      if (geometry.row?.bottom > geometry.height + 1) errors.push('Production composer falls below the viewport');
    }
    if (geometry.overflow) errors.push('App overflows horizontally');
  } catch (error) {
    errors.push(String(error).slice(0, 1200));
    await writeFile(path.join(report, name + '-body.txt'), (await page.locator('body').innerText()).slice(0, 20000));
  } finally {
    await page.screenshot({path: path.join(report, name + '.png')});
    await context.close();
  }
  return {name, geometry, contentLayout, errors};
}

let browser;
const results = [];
try {
  for (let i = 0; i < 100; i++) {
    try {if ((await fetch(`http://127.0.0.1:${port}/lab/status`)).ok) break;} catch {}
    await new Promise(r => setTimeout(r, 100));
  }
  browser = await chromium.launch({headless: true});
  for (const size of [{width: 1280, height: 900}, {width: 390, height: 844}, {width: 320, height: 568}]) {
    for (const scene of ['gui', 'codex', 'terminal', 'content', ...(size.width < 500 ? ['page'] : []), 'list']) {
      results.push(await inspect(browser, scene, size));
      await writeFile(path.join(report, 'results.json'), JSON.stringify(results, null, 2));
    }
  }
  console.log(JSON.stringify(results.map(({name, errors}) => ({name, errors}))));
  if (results.some(result => result.errors.length)) process.exitCode = 1;
} finally {
  await browser?.close();
  server.kill('SIGTERM');
}
