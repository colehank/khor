import {chromium} from 'playwright-core';
import {spawn} from 'node:child_process';
import {mkdir, writeFile} from 'node:fs/promises';
import path from 'node:path';
const report = process.env.LAB_REPORT;
await mkdir(report, {recursive: true});
const port = 23918;
const server = spawn('python3', ['-m', 'khor_lab.preview', process.env.LAB_BUNDLE, '--port', String(port)], {
  cwd: path.resolve('../..'), env: {...process.env, PYTHONPATH: 'scripts'}, stdio: 'inherit',
});
let browser;
const results = [];
try {
  for (let i = 0; i < 100; i++) {
    try {if ((await fetch(`http://127.0.0.1:${port}/lab/status`)).ok) break;} catch {}
    await new Promise(r => setTimeout(r, 100));
  }
  browser = await chromium.launch({headless: true});
  for (const size of [{width: 1280, height: 900}, {width: 390, height: 844}, {width: 320, height: 568}]) {
    for (const scene of ['gui', 'codex', 'terminal', 'content', 'list']) {
      const context = await browser.newContext({viewport: size, isMobile: size.width < 500, hasTouch: size.width < 500});
      const page = await context.newPage();
      const errors = [];
      page.on('pageerror', error => errors.push(String(error)));
      page.on('console', message => {if (message.type() === 'error' && message.text().includes('[Lab fixture]')) errors.push(message.text());});
      // The same real App/adapter; only select data, never replace a component or layout.
      await page.route('**/lab/runtime*', route => route.fulfill({contentType: 'application/json', body: JSON.stringify({mode: 'frontend', scene, tree: 'browser-check', revision: 'one'})}));
      await page.goto(`http://127.0.0.1:${port}/app/?theme=dark`);
      await page.waitForSelector(scene === 'list' ? '[data-list]' : scene === 'terminal' ? '[data-terminal]' : '[data-chat-box]');
      if (size.width < 500 && ['gui', 'codex'].includes(scene)) await page.waitForSelector('[data-session-compose-row]');
      if (scene === 'content') await page.waitForSelector('[data-session-content-pane][aria-hidden="false"]');
      await page.waitForTimeout(800);
      const geometry = await page.evaluate(() => {
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
      const name = `${scene}-${size.width}x${size.height}`;
      await page.screenshot({path: path.join(report, name + '.png')});
      results.push({name, geometry, errors});
      await writeFile(path.join(report, 'results.json'), JSON.stringify(results, null, 2));
      await context.close();
    }
  }
  await writeFile(path.join(report, 'results.json'), JSON.stringify(results, null, 2));
  console.log(JSON.stringify(results.map(({name, errors}) => ({name, errors}))));
  if (results.some(result => result.errors.length)) process.exitCode = 1;
} finally {
  await browser?.close();
  server.kill('SIGTERM');
}
