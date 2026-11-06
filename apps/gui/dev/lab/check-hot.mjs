import assert from 'node:assert/strict';
import {spawn, execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {mkdir, readFile, writeFile} from 'node:fs/promises';
import path from 'node:path';
import {chromium} from 'playwright-core';

const root = path.resolve('../..'), port = 23919;
if (execFileSync('git', ['rev-parse', '--show-toplevel'], {cwd: root, encoding: 'utf8'}).trim() !== root) {
  execFileSync('git', ['init', '-q'], {cwd: root});
  execFileSync('git', ['add', '--all'], {cwd: root});
  execFileSync('git', ['-c', 'user.name=Lab', '-c', 'user.email=lab@localhost', 'commit', '-qm', 'Materialized developer source'], {cwd: root});
}
const report = path.resolve(process.env.LAB_REPORT);
const cache = path.join(report, 'hot-cache');
const home = path.join(cache, createHash('sha256').update(root).digest('hex').slice(0, 16) + `-${port}-dev`);
await mkdir(home, {recursive: true});
// Generated contracts were produced by this exact source's frontend job.
const contract = execFileSync('python3', ['-c', 'from khor_lab.contract import generated_digest; from pathlib import Path; print(generated_digest(Path.cwd()))'], {cwd: root, env: {...process.env, PYTHONPATH: 'scripts'}, encoding: 'utf8'}).trim();
await writeFile(path.join(home, 'generated.json'), JSON.stringify({contract}));
const css = path.resolve('src/styles/app.css'), original = await readFile(css, 'utf8');
const server = spawn('python3', ['scripts/dev.py', 'lab', 'dev', '--builder', 'local', '--mode', 'frontend', '--scene', 'gui', '--port', String(port), '--cache-root', cache], {cwd: root, stdio: 'inherit'});
let browser;
try {
  let ready = false;
  for (let i = 0; i < 200; i++) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/lab/runtime`);
      if (response.ok) {assert.equal((await response.json()).mutable, true); ready = true; break;}
    } catch {}
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  assert.ok(ready, 'Source development did not start');
  browser = await chromium.launch({headless: true});
  const page = await browser.newPage({viewport: {width: 390, height: 844}, isMobile: true, hasTouch: true});
  await page.goto(`http://127.0.0.1:${port}/app/`);
  const input = page.locator('[data-chat-box] textarea').first();
  await input.fill('保留这段草稿 HMR');
  await page.evaluate(() => {window.__labHotIdentity = 'same-document';});
  await writeFile(css, original + '\n:root { --lab-hot-proof: received; }\n');
  await page.waitForFunction(() => getComputedStyle(document.documentElement).getPropertyValue('--lab-hot-proof').trim() === 'received');
  assert.equal(await page.evaluate(() => window.__labHotIdentity), 'same-document', 'CSS update reloaded the document');
  assert.equal(await input.inputValue(), '保留这段草稿 HMR', 'CSS update lost the production composer draft');
  await writeFile(path.join(report, 'hot.json'), JSON.stringify({passed: true, mutable: true, productionApp: true, cssUpdatedWithoutReload: true, draftPreserved: true}, null, 2));
} finally {
  await writeFile(css, original);
  await browser?.close();
  server.kill('SIGTERM');
  if (server.exitCode === null) await new Promise(resolve => {server.once('exit', resolve); setTimeout(resolve, 5000);});
}
