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

async function inspect(browser, scene, size) {
  const context = await browser.newContext({viewport: size, isMobile: size.width < 500, hasTouch: size.width < 500});
  const page = await context.newPage(), errors = [];
  const name = `${scene}-${size.width}x${size.height}`;
  let geometry = null;
  page.on('pageerror', error => errors.push(String(error)));
  page.on('console', message => {
    if (message.type() === 'error' && message.text().includes('[Lab fixture]')) errors.push(message.text().split('\n')[0]);
  });
  try {
    // Select data only; every layout and action belongs to the production App.
    await page.route('**/lab/runtime*', route => route.fulfill({contentType: 'application/json', body: JSON.stringify({mode: 'frontend', scene, tree: 'browser-check', revision: 'one'})}));
    await page.goto(`http://127.0.0.1:${port}/app/?theme=dark`);
    const selector = scene === 'list' ? '[data-list]' : scene === 'terminal' ? '[data-terminal]'
      : scene === 'content' ? '.cm-editor, [data-file-preview]' : '[data-chat-box]';
    await page.waitForSelector(selector);
    if (scene === 'content') {
      const document = await page.locator('.cm-editor, [data-file-preview]').first().innerText();
      if (!document.includes('fixture data')) errors.push('Actual file body is missing');
    }
    if (size.width < 500 && ['gui', 'codex'].includes(scene)) await page.waitForSelector('[data-session-compose-row]');
    await page.waitForTimeout(800);
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
  return {name, geometry, errors};
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
    for (const scene of ['gui', 'codex', 'terminal', 'content', 'list']) {
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
