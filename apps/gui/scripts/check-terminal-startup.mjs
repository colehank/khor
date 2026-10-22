// Real app, private node and PTY. The probe uses Codex's 250 ms color
// deadline without a model, login, or a request to any external service.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { appendFileSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
import { fakeHome, fakeUsage } from './fake-home.mjs';

const gui = fileURLToPath(new URL('..', import.meta.url));
const target = process.env.CARGO_TARGET_DIR || resolve(gui, '../../target');
const scratch = mkdtempSync(join(tmpdir(), 'khor-terminal-startup-'));
const port = Number(process.env.STARTUP_VITE_PORT || 23946);
const bridgePort = Number(process.env.STARTUP_BRIDGE_PORT || 23947);
const children = [], sessions = [];
const delay = ms => new Promise(r => setTimeout(r, ms));
const quote = s => `'${s.replaceAll("'", "'\\''")}'`;
let browser, page, usage;
const api = async (cmd, args = {}) => {
  const response = await fetch(`http://127.0.0.1:${bridgePort}/${cmd}`, {
    method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify(args), signal: AbortSignal.timeout(15000),
  });
  const value = await response.json();
  if (!response.ok) throw Error(JSON.stringify(value));
  return value;
};
const until = async (fn, label) => {
  const end = Date.now() + 30000;
  while (Date.now() < end) {
    try { if (await fn()) return; } catch {}
    await delay(50);
  }
  throw Error(`Timed out: ${label}`);
};
const run = (cmd, args, env) => {
  const child = spawn(cmd, args, { cwd: gui, env, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
  children.push(child);
  child.stdout.on('data', data => appendFileSync(join(scratch, 'services.log'), data));
  child.stderr.on('data', data => process.stderr.write(data));
};

try {
  usage = await fakeUsage();
  const env = { ...process.env, ...fakeHome(join(scratch, 'home'), usage.url),
    KHOR_HOME: join(scratch, 'node'), KHOR_NAME: 'startup-check', SHELL: '/bin/sh',
    BRIDGE_PORT: String(bridgePort), KHOR_FACE_GATE: '1' };
  delete env.KHOR_SESSION;
  for (const name of ['bridge', 'khor']) copyFileSync(join(target, 'debug', name), join(scratch, name));
  run(join(scratch, 'bridge'), [], env);
  await until(() => api('devices').then(devices => devices.length > 0), 'private bridge');
  execFileSync(join(scratch, 'khor'), ['config', 'set', 'theme', 'dark'], { env, stdio: 'ignore' });

  const fixture = join(scratch, 'probe.py');
  writeFileSync(fixture, `import json, os, select, sys, termios, time, tty
from pathlib import Path
result = Path(sys.argv[1])
with result.with_suffix('.count').open('a') as f: f.write('x')
fd = sys.stdin.fileno()
saved = termios.tcgetattr(fd)
seen = b''
try:
    tty.setraw(fd)
    os.write(1, b'\\x1b]10;?\\x1b\\\\\\x1b]11;?\\x1b\\\\')
    deadline = time.monotonic() + 0.250
    while time.monotonic() < deadline:
        if not select.select([fd], [], [], max(0, deadline - time.monotonic()))[0]: break
        seen += os.read(fd, 4096)
        if b']10;rgb:' in seen and b']11;rgb:' in seen: break
finally:
    termios.tcsetattr(fd, termios.TCSANOW, saved)
ok = b']10;rgb:' in seen and b']11;rgb:' in seen
result.write_text(json.dumps({'ok': ok, 'reply': seen.hex()}))
os.write(1, b'\\r\\n\\x1b[48;2;80;80;80m INPUT_BACKGROUND \\x1b[0m\\r\\n' + (b'PROBE_OK' if ok else b'PROBE_TIMEOUT') + b'\\r\\n')
`);
  const python = execFileSync('which', ['python3'], { encoding: 'utf8' }).trim();
  const result = join(scratch, 'result.json');
  const machine = (await api('devices')).find(device => device.me).name;
  const args = { machine, cwd: scratch, command: `${quote(python)} ${quote(fixture)} ${quote(result)}`,
    title: 'Terminal startup', cols: 100, rows: 35, token: crypto.randomUUID() };
  const id = await api('open_shell', args);
  sessions.push(id);
  assert.equal(await api('open_shell', args), id, 'Creation retry must keep the same session');
  // Hold the viewer back longer than the program's startup probe deadline.
  await delay(750);
  assert(!existsSync(join(scratch, 'result.count')), 'Startup ran before any terminal attached');
  await api('term_open', { id, cols: 100, rows: 35, history: 2000 });
  await api('term_attach', { id });
  await delay(500);
  assert(!existsSync(join(scratch, 'result.count')), 'Socket attachment is not renderer readiness');
  console.log('PASS delayed creation and socket attachment keep startup pending');

  run(join(gui, 'node_modules/.bin/vite'), ['--host', '127.0.0.1', '--port', String(port), '--strictPort'], env);
  await until(() => fetch(`http://127.0.0.1:${port}/`).then(r => r.ok), 'private app');
  browser = await chromium.launch({ channel: 'chrome', headless: false, args: ['--no-proxy-server'] });
  const context = await browser.newContext({ viewport: { width: 1280, height: 860 } });
  await context.addInitScript(id => {
    if (!localStorage.getItem('khor.sessions.tabs')) localStorage.setItem('khor.sessions.tabs', JSON.stringify({ tabs: [id], active: id }));
    localStorage.setItem('khor.place', JSON.stringify({ landing: 'sessions', screen: 'detail' }));
    localStorage.setItem('khor.lang', 'zh');
  }, id);
  page = await context.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(`http://127.0.0.1:${port}/?bridge=${bridgePort}`, { waitUntil: 'domcontentloaded' });
  await until(() => existsSync(result), '250 ms color probe');
  assert(JSON.parse(readFileSync(result, 'utf8')).ok, 'Startup color queries did not receive live replies');
  await until(() => page.locator('[data-terminal]').evaluateAll(elements => elements.some(el => {
    const term = el.__xterm;
    if (!term) return false;
    for (let i = 0; i < term.buffer.active.length; i++) {
      const line = term.buffer.active.getLine(i), text = line.translateToString();
      if (text.includes('INPUT_BACKGROUND')) return line.getCell(text.indexOf('INPUT_BACKGROUND')).getBgColor() === 0x505050;
    }
    return false;
  })), 'rendered explicit input background');
  await page.screenshot({ path: join(scratch, 'desktop.png') });
  await page.locator('[data-terminal]').evaluateAll(elements => Promise.all(elements.map(el => el.__reopen?.())));
  await page.reload({ waitUntil: 'domcontentloaded' });
  await until(() => page.locator('[data-terminal]').evaluateAll(elements => elements.some(el => el.__xterm)), 'reloaded terminal');
  await api('term_ready', { id });
  assert.equal(readFileSync(join(scratch, 'result.count'), 'utf8'), 'x', 'Reopen, reload and repeated ready must not run startup again');
  await page.setViewportSize({ width: 390, height: 844 });
  if (!await page.locator('[data-terminal]:visible').count()) await page.getByText('Terminal startup', { exact: true }).first().click();
  await page.screenshot({ path: join(scratch, 'mobile.png') });
  if (process.env.STARTUP_CODEX) {
    const home = join(scratch, 'codex'), thread = crypto.randomUUID();
    const now = new Date().toISOString(), day = now.slice(0, 10).replaceAll('-', '/');
    const folder = join(home, 'sessions', day);
    mkdirSync(folder, { recursive: true });
    const rows = [
      { type: 'session_meta', payload: { id: thread, timestamp: now, cwd: scratch,
        originator: 'codex_cli_rs', cli_version: execFileSync(process.env.STARTUP_CODEX, ['--version'], { encoding: 'utf8' }).trim().split(' ').at(-1),
        source: 'cli', model_provider: 'fixture', base_instructions: { text: 'Private display fixture. Do not call tools.' } } },
      { type: 'response_item', payload: { type: 'message', role: 'user', content: [{ type: 'input_text', text: 'BACKGROUND_FIXTURE' }] } },
      { type: 'event_msg', payload: { type: 'user_message', message: 'BACKGROUND_FIXTURE', images: [], local_images: [] } },
      { type: 'response_item', payload: { type: 'message', role: 'assistant', content: [{ type: 'output_text', text: 'Display fixture ready.' }] } },
      { type: 'event_msg', payload: { type: 'agent_message', message: 'Display fixture ready.', phase: 'final_answer' } },
    ];
    writeFileSync(join(folder, `rollout-${now.replaceAll(':', '-')}-${thread}.jsonl`), rows.map(row => JSON.stringify({ timestamp: now, ...row })).join('\n') + '\n');
    writeFileSync(join(home, 'config.toml'), `model="fixture"\nmodel_provider="fixture"\n[tui]\nscreen_reader_detection_done=true\n[model_providers.fixture]\nname="fixture"\nbase_url="http://127.0.0.1:9/v1"\nwire_api="responses"\nenv_key="KHOR_STARTUP_FIXTURE_KEY"\n[projects.${JSON.stringify(scratch)}]\ntrust_level="trusted"\n`);
    const codex = await api('open_shell', { ...args, token: crypto.randomUUID(), title: 'Codex startup',
      command: `env CODEX_HOME=${quote(home)} KHOR_STARTUP_FIXTURE_KEY=fixture ${quote(process.env.STARTUP_CODEX)} resume ${thread}` });
    sessions.push(codex);
    await delay(750);
    await page.setViewportSize({ width: 1280, height: 860 });
    await page.evaluate(id => localStorage.setItem('khor.sessions.tabs', JSON.stringify({ tabs: [id], active: id })), codex);
    await page.reload({ waitUntil: 'domcontentloaded' });
    const hasComposerBackground = () => page.locator('[data-terminal]:visible').evaluateAll(elements => elements.some(el => {
      const term = el.__xterm;
      if (!term) return false;
      for (let i = 0; i < term.buffer.active.length; i++) {
        const line = term.buffer.active.getLine(i), text = line.translateToString(), at = text.indexOf('Ask Codex');
        if (at >= 0 && !line.getCell(at).isBgDefault()) return true;
      }
      return false;
    }));
    await until(hasComposerBackground, 'installed Codex composer background after delayed attachment');
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await page.screenshot({ path: join(scratch, 'codex-desktop.png') });
    await page.setViewportSize({ width: 390, height: 844 });
    if (!await page.locator('[data-terminal]:visible').count()) await page.getByText('Codex startup', { exact: true }).first().click();
    await until(hasComposerBackground, 'installed Codex composer background after mobile resize');
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await page.screenshot({ path: join(scratch, 'codex-mobile.png') });
    console.log('PASS installed Codex resumes with an explicit composer background; no prompt submitted');
  }
  assert.deepEqual(errors, []);
  console.log(`PASS live color replies, painted background and exactly-once startup; evidence ${scratch}`);
} catch (error) {
  await page?.screenshot({ path: join(scratch, 'failed.png'), timeout: 5000 }).catch(() => {});
  console.error(`Evidence: ${scratch}`);
  throw error;
} finally {
  for (const id of sessions) await api('close_session', { id }).catch(() => {});
  await browser?.close();
  for (const child of children) { try { process.kill(-child.pid, 'SIGKILL'); } catch {} }
  for (const name of ['bridge', 'khor']) rmSync(join(scratch, name), { force: true });
  await usage?.close();
}
