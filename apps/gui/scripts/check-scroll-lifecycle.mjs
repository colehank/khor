// Production CommitPage, ChatView and MachineChat through an isolated transport.
// The viewport seam models keyboard geometry; it is not native iOS acceptance.
// QUALITY_URL reuses a preview without taking ownership of its process.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium, webkit } from 'playwright-core';
import { qualityCommitShow } from "./quality-fixture.mjs";

const gui = fileURLToPath(new URL('..', import.meta.url));
const repo = join(gui, '../..');
const port = Number(process.env.QUALITY_PORT ?? 23821);
const base = (process.env.QUALITY_URL ?? `http://127.0.0.1:${port}`).replace(/\/$/, '');
const out = process.env.QUALITY_OUT ?? join(tmpdir(), `khor-scroll-lifecycle-${process.pid}`);
const headless = process.env.QUALITY_HEADLESS === '1';
const engines = (process.env.QUALITY_ENGINES ?? 'chrome').split(',');
const only = (process.env.QUALITY_ONLY ?? '').split(',').filter(Boolean);
const names = ['lock-unavailable', 'lock-pin-failure', 'commit-mouse', 'commit-touch', 'commit-keyboard', 'commit-escape', 'lab-acp-blur', 'chat-acp-tail', 'chat-acp-history', 'chat-acp-input-tail', 'chat-acp-input-history', 'chat-acp-blur-restore', 'chat-khor-tail', 'chat-khor-history', 'chat-khor-input-tail', 'chat-khor-input-history', 'chat-khor-blur-restore'];
for (const name of engines) assert(['chrome', 'webkit'].includes(name), `Unknown engine: ${name}`);
for (const name of only) assert(names.some(candidate => candidate === name || candidate.startsWith(`${name}-`)), `Unknown scenario: ${name}`);
mkdirSync(out, { recursive: true });
const hash = value => createHash('sha256').update(value).digest('hex');
const files = ['src/hooks/use-lock.ts', 'src/views/LockScreen.tsx', 'src/App.tsx', '../../crates/catalog/zh.toml', '../../crates/catalog/en.toml', 'src/hooks/use-chat-scroll.ts', 'src/views/CommitPage.tsx', 'scripts/check-scroll-lifecycle.mjs', 'scripts/quality-fixture.mjs', 'scripts/quality-preview.vite.ts', 'scripts/ai-elements-preview.vite.ts', 'scripts/session-navigation-preview.vite.ts', 'lab/quality.tsx', 'lab/quality.css', 'lab/quality.html'];
const report = {
  started: new Date().toISOString(), base, headless,
  head: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repo, encoding: 'utf8' }).trim(),
  diffHash: hash(execFileSync('git', ['diff', '--binary', 'HEAD'], { cwd: repo })),
  files: Object.fromEntries(files.map(file => [file, hash(readFileSync(join(gui, file)))])),
  boundary: 'Production UI, fixture transport, synthetic visualViewport; no native keyboard or backend acceptance.',
  results: [],
};
const save = () => writeFileSync(join(out, 'results.json'), JSON.stringify(report, null, 2));
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
let server, browser, serverLog = '';

async function startPreview() {
  if (process.env.QUALITY_URL) return;
  const config = { configFile: join(gui, 'scripts/quality-preview.vite.ts'), cacheDir: join(out, 'vite-cache'), server: { host: '127.0.0.1', port, strictPort: true } };
  server = spawn(process.execPath, ['--input-type=module', '-e', `
    import { createServer } from 'vite';
    const server = await createServer(JSON.parse(process.argv[1]));
    await server.listen(); console.log('QUALITY_READY');
    const stop = async () => { await server.close(); process.exit(0); };
    process.on('SIGTERM', stop); process.on('SIGINT', stop);
  `, JSON.stringify(config)], { cwd: gui, env: { ...process.env, KHOR_FACE_GATE: '1' }, stdio: ['ignore', 'pipe', 'pipe'] });
  let ready = false, startupError;
  server.on('error', error => { startupError = error; });
  for (const stream of [server.stdout, server.stderr]) stream.on('data', data => {
    serverLog = (serverLog + data).slice(-30_000);
    ready ||= String(data).includes('QUALITY_READY');
  });
  const deadline = Date.now() + 30_000;
  while (!ready) {
    if (startupError) throw startupError;
    if (server.exitCode !== null) throw new Error(`Preview exited: ${serverLog}`);
    assert(Date.now() < deadline, `Preview did not start: ${serverLog}`);
    await sleep(100);
  }
}

async function stopPreview() {
  if (!server || server.exitCode !== null) return;
  await new Promise(resolve => {
    const timer = setTimeout(() => server.kill('SIGKILL'), 4000);
    server.once('exit', () => { clearTimeout(timer); resolve(); });
    server.kill('SIGTERM');
  });
}

async function frames(page, count = 3) {
  await page.evaluate(async count => {
    for (let i = 0; i < count; i++) await new Promise(requestAnimationFrame);
  }, count);
}

// Samples outlive the close animation and include every frame in the quiet
// period. Nothing here focuses, clicks, scrolls, or calls production methods.
async function samples(page, kind, target = null, duration = 700) {
  return page.evaluate(async ({ kind, target, duration }) => {
    const result = [], from = performance.now();
    do {
      await new Promise(requestAnimationFrame);
      const box = document.querySelector(kind === 'commit' ? '[data-commit-scroll]' : kind);
      if (!box) throw new Error(`Missing scroll surface: ${kind}`);
      const bounds = box.getBoundingClientRect();
      const header = kind === 'commit' ? document.querySelector('[data-commit-head]') : null;
      const covered = header ? Math.max(0, header.getBoundingClientRect().bottom - bounds.top) : 0;
      const element = target ? [...box.querySelectorAll('[data-commit-file]')].find(e => e.dataset.commitFile === target) : null;
      const active = document.activeElement;
      result.push({
        at: Math.round(performance.now() - from), top: box.scrollTop,
        height: box.scrollHeight, view: box.clientHeight,
        gap: box.scrollHeight - box.clientHeight - box.scrollTop,
        offset: element ? element.getBoundingClientRect().top - bounds.top : null,
        covered,
        focus: active?.hasAttribute('data-commit-jump') ? 'trigger' : active?.getAttribute('data-commit-file') ?? active?.tagName,
        focusInTarget: !!element && (active === element || element.contains(active)),
      });
    } while (performance.now() - from < duration);
    return result;
  }, { kind, target, duration });
}

function assertTail(readings, label) {
  assert(readings.length > 3, `${label}: no observation window`);
  assert(readings.every(s => Math.abs(s.gap) <= 2), `${label}: tail left the viewport: ${JSON.stringify(readings.filter(s => Math.abs(s.gap) > 2).slice(0, 4))}`);
}

async function commitCase(page, mode, evidence) {
  const target = qualityCommitShow.files[5].path;
  await page.goto(`${base}/lab/commit.html?id=${qualityCommitShow.commit.id}`);
  await page.waitForFunction(count => document.querySelectorAll('[data-commit-diff] .cm-editor').length === count, qualityCommitShow.files.length);
  const trigger = page.locator('[data-commit-jump]');
  const menu = page.locator('[data-commit-jump-menu]');
  await frames(page, 5);
  evidence.before = (await samples(page, 'commit', target, 100)).at(-1);
  assert(evidence.before.offset > evidence.before.view, 'Fixture target must start below the viewport');
  await page.evaluate(() => {
    window.__qualityTrace = [];
    const box = document.querySelector('[data-commit-scroll]');
    const record = event => window.__qualityTrace.push({ at: performance.now(), event: event.type, top: box.scrollTop, focus: document.activeElement?.hasAttribute('data-commit-jump') ? 'trigger' : document.activeElement?.tagName });
    box.addEventListener('scroll', record); document.addEventListener('focusin', record);
  });
  if (mode === 'touch') await trigger.tap();
  else if (mode === 'keyboard' || mode === 'escape') {
    // Reach the trigger as a keyboard user, without calling element.focus().
    for (let i = 0; i < 30; i++) {
      // WebKit on macOS uses Option-Tab to include buttons in traversal.
      const optionTab = process.platform === 'darwin' && page.context().browser().browserType().name() === 'webkit';
      await page.keyboard.press(optionTab ? 'Alt+Tab' : 'Tab');
      if (await trigger.evaluate(el => el === document.activeElement)) break;
    }
    assert(await trigger.evaluate(el => el === document.activeElement), 'Tab must reach the jump trigger');
    await page.keyboard.press('Enter');
  } else await trigger.click();
  await menu.waitFor({ state: 'visible' });
  if (mode === 'escape') {
    await page.keyboard.press('Escape');
  } else if (mode === 'keyboard') {
    await page.waitForFunction(() => document.activeElement?.hasAttribute('data-commit-jump-item'));
    await page.keyboard.press('Home');
    await page.waitForFunction(path => document.activeElement?.getAttribute('data-commit-jump-item') === path, qualityCommitShow.files[0].path);
    for (let i = 1; i <= 5; i++) {
      await page.keyboard.press('ArrowDown');
      await page.waitForFunction(path => document.activeElement?.getAttribute('data-commit-jump-item') === path, qualityCommitShow.files[i].path);
    }
    assert.equal(await page.evaluate(() => document.activeElement?.getAttribute('data-commit-jump-item')), target, 'keyboard selection reaches the sixth file');
    await page.keyboard.press('Enter');
  } else {
    const item = page.locator('[data-commit-jump-item]').nth(5);
    if (mode === 'touch') await item.tap(); else await item.click();
  }
  await menu.waitFor({ state: 'detached' });
  evidence.samples = await samples(page, 'commit', target);
  evidence.trace = await page.evaluate(() => window.__qualityTrace);
  if (mode === 'escape') {
    assert(evidence.samples.every(s => s.focus === 'trigger'), 'Escape restores and retains trigger focus');
    assert(evidence.samples.every(s => Math.abs(s.top - evidence.before.top) <= 2), 'Escape does not navigate or scroll');
    return;
  }
  assert(evidence.samples.every(s => s.top > 100 && s.offset >= s.covered - 2 && s.offset < s.view && (Math.abs(s.offset - s.covered) <= 2 || Math.abs(s.gap) <= 2)), `Selected file must remain visible below the header after menu exit: ${JSON.stringify(evidence.samples.slice(-3))}`);
  assert(evidence.samples.every(s => s.focusInTarget), 'Selection transfers and retains focus in the selected file');
  assert.equal(new Set(evidence.samples.map(s => s.focus)).size, 1, 'Focus remains stable after menu exit');
}

async function labBlurCase(page, evidence) {
  const reset = await page.request.post(`${base}/__ai_preview/reset`);
  assert(reset.ok(), 'Isolated Lab reset succeeds');
  await page.goto(`${base}/lab/quality.html`);
  await page.getByRole('button', { name: '载入长回复', exact: true }).click();
  const handle = await page.locator('iframe[title="对话验收预览"]').elementHandle();
  const frame = await handle.contentFrame();
  await handle.dispose();
  assert(frame, 'The real Lab must have its embedded App');
  const selector = '[data-chat-scroll]';
  await frame.getByText('长回复结束 · 检查这句话是否仍在输入框上方。', { exact: true }).waitFor({ state: 'attached' });
  const bottom = frame.locator('[data-chat-bottom]');
  if (await bottom.isVisible()) await bottom.click();
  await frame.waitForFunction(() => {
    const box = document.querySelector('[data-chat-scroll]');
    return box && box.scrollHeight - box.clientHeight > 800 && Math.abs(box.scrollHeight - box.clientHeight - box.scrollTop) <= 2;
  });
  const input = frame.locator('[data-composer-text]');
  const draft = '键盘验收草稿：这段文字不发送';
  await input.fill(draft);
  evidence.draftBeforeLayout = await frame.locator(selector).evaluate(box => ({
    at: performance.now(), top: box.scrollTop, height: box.scrollHeight, view: box.clientHeight,
    gap: box.scrollHeight - box.clientHeight - box.scrollTop,
    focus: document.activeElement?.tagName,
  }));
  await frames(frame);
  evidence.focusBefore = await frame.evaluate(() => ({ documentHasFocus: document.hasFocus(), activeIsComposer: document.activeElement?.matches('[data-composer-text]') }));
  assert(evidence.focusBefore.documentHasFocus && evidence.focusBefore.activeIsComposer, 'Typing must really focus the embedded composer before focus leaves');
  evidence.draftBefore = await samples(frame, selector, null, 350);
  assertTail(evidence.draftBefore, 'Lab draft before external keyboard control');
  await page.getByRole('button', { name: '升起模拟键盘', exact: true }).click();
  evidence.focus = await frame.evaluate(() => ({
    documentHasFocus: document.hasFocus(),
    activeIsComposer: document.activeElement?.matches('[data-composer-text]'),
    activeTag: document.activeElement?.tagName,
  }));
  assert.equal(evidence.focus.documentHasFocus, false, 'The outer Lab control must really take focus from the iframe');
  await frames(frame);
  evidence.keyboardOpen = await samples(frame, selector, null, 1200);
  assert(evidence.keyboardOpen.at(-1).view < evidence.draftBefore.at(-1).view - 100, 'The Lab control must really reduce the inner App viewport');
  assertTail(evidence.keyboardOpen, 'Lab keyboard after iframe focus leaves');
  assert.equal(await input.inputValue(), draft, 'Changing the keyboard preserves the draft');
  await page.getByRole('button', { name: '收起模拟键盘', exact: true }).click();
  await frames(frame);
  evidence.keyboardClose = await samples(frame, selector, null, 1200);
  assertTail(evidence.keyboardClose, 'Lab keyboard closed while iframe is unfocused');
  assert.equal(await input.inputValue(), draft, 'Closing the keyboard preserves the draft');
}

const paragraph = (kind, index) => `滚动验收 ${kind} ${index}：这段内容用于检查键盘展开后，读者选择的位置与最新消息仍然可见。`;
const note = value => ({ kind: 'note', update: { update: { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: value } } } });

async function keyboard(page, cover, evidence, label) {
  const animation = await page.evaluate(async cover => {
    const viewport = window.visualViewport;
    if (!window.__qualityViewport) {
      const state = { cover: 0 };
      Object.defineProperty(viewport, 'height', { configurable: true, get: () => window.innerHeight - state.cover });
      window.__qualityViewport = state;
    }
    const state = window.__qualityViewport, from = state.cover, values = [];
    for (let step = 1; step <= 12; step++) {
      state.cover = Math.round(from + (cover - from) * step / 12);
      viewport.dispatchEvent(new Event('resize'));
      await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
      const box = document.querySelector('[data-chat-scroll], [data-machine-chat-scroll]');
      values.push({ cover: state.cover, top: box.scrollTop, height: box.scrollHeight, view: box.clientHeight, gap: box.scrollHeight - box.clientHeight - box.scrollTop });
    }
    return values;
  }, cover);
  evidence[label] = animation;
  await frames(page);
}

// A reader's wheel can share a frame with a keyboard/composer resizing the
// surface. Inject only geometry during the trusted event, never a scroll value.
async function inputDuringResize(page, selector, evidence) {
  evidence.inputBefore = (await samples(page, selector, null, 150)).at(-1);
  await page.locator(selector).evaluate(box => {
    window.__qualityWheel = null;
    box.addEventListener('wheel', event => {
      window.__qualityWheel = { trusted: event.isTrusted, top: box.scrollTop, view: box.clientHeight, deltaY: event.deltaY };
      const height = box.clientHeight;
      box.style.flex = 'none';
      box.style.height = `${height - 80}px`;
    }, { passive: true, once: true });
  });
  const bounds = await page.locator(selector).boundingBox();
  await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  await page.mouse.wheel(0, -400);
  evidence.inputFrames = await samples(page, selector, null, 650);
  evidence.wheel = await page.evaluate(() => window.__qualityWheel);
  assert(evidence.wheel?.trusted, 'The concurrent resize must be driven by a real wheel event');
  const settled = evidence.inputFrames.filter(sample => sample.at >= 200);
  assert(settled.length > 3, 'Concurrent input must have a sustained observation window');
  assert(settled.every(sample => sample.view < evidence.inputBefore.view - 70), 'The input frame must actually reduce the available height');
  assert(settled.every(sample => sample.top < evidence.inputBefore.top - 200), `A resize must not undo the reader's upward wheel: ${JSON.stringify({ before: evidence.inputBefore, wheel: evidence.wheel, after: settled.at(-1) })}`);
}

async function chatCase(page, kind, evidence, phase = 'tail') {
  const session = kind === 'acp' ? 'gui/ai-elements-review' : 'wide/all';
  const selector = kind === 'acp' ? '[data-chat-scroll]' : '[data-machine-chat-scroll]';
  const bottom = kind === 'acp' ? '[data-chat-bottom]' : '[data-machine-chat-bottom]';
  const rows = [];
  let next = 0;
  const add = async (count, waitForBody = true) => {
    let last;
    if (kind === 'acp') {
      const text = Array.from({ length: count }, () => (last = paragraph(kind, ++next))).join('\n\n');
      const delivered = page.waitForResponse(async response => {
        if (!/\/(chat_poll|chat_wait)$/.test(new URL(response.url()).pathname)) return false;
        const value = await response.json();
        return value.frames?.some(frame => frame.kind === 'content_changed');
      });
      const response = await page.request.post(`${base}/__ai_preview/acp`, { data: { id: session, frames: [note(`\n\n${text}`)] } });
      assert(response.ok(), 'ACP fixture accepts new frames');
      await delivered;
    } else {
      for (let i = 0; i < count; i++) {
        last = paragraph(kind, ++next);
        rows.push({ id: `quality-${next}`, at_ms: 1792900800000 + next * 1000, from: '验收消息', from_me: false, to: [], text: last, files: null, note: null, unknown: null, retracted: false, command: null });
      }
    }
    if (waitForBody || kind !== 'acp') await page.getByText(last, { exact: true }).waitFor({ state: 'attached' });
    return last;
  };
  const reset = await page.request.post(`${base}/__ai_preview/reset`);
  assert(reset.ok(), 'Isolated fixture reset succeeds');
  if (kind === 'khor') await page.route('**/machine_log', async route => {
    const upstream = await route.fetch(), value = await upstream.json();
    await route.fulfill({ response: upstream, json: { ...value, messages: [...value.messages, ...rows] } });
  });
  await page.goto(`${base}/?${new URLSearchParams({ phone: '1', labSession: session, labTabs: session })}`, { waitUntil: 'domcontentloaded', timeout: 45_000 });
  await page.locator(selector).waitFor();
  await add(60);
  await page.waitForFunction(selector => { const box = document.querySelector(selector); return box && box.scrollHeight - box.clientHeight > 1000; }, selector);
  const toBottom = page.locator(bottom);
  if (await toBottom.isVisible()) await toBottom.click();
  await page.waitForFunction(selector => { const box = document.querySelector(selector); return Math.abs(box.scrollHeight - box.clientHeight - box.scrollTop) <= 2; }, selector);
  evidence.before = await samples(page, selector, null, 200);
  assertTail(evidence.before, 'Initial live tail');
  const input = page.locator('[data-composer-text]');
  await input.click();
  if (phase === 'input-tail') return inputDuringResize(page, selector, evidence);
  if (phase === 'blur-restore') {
    const oldTop = evidence.before.at(-1).top;
    await keyboard(page, 300, evidence, 'blurKeyboardOpen');
    await input.fill('draft words '.repeat(24));
    await frames(page);
    evidence.blurDraftOpen = await samples(page, selector, null, 350);
    assertTail(evidence.blurDraftOpen, 'Long draft while the composer owns layout');
    await input.evaluate(element => element.blur());
    await keyboard(page, 0, evidence, 'blurKeyboardClose');
    evidence.blurBeforeRestoration = await samples(page, selector, null, 350);
    assertTail(evidence.blurBeforeRestoration, 'Layout settles after the composer loses focus');
    evidence.passiveRestoration = await page.locator(selector).evaluate((box, oldTop) => {
      const before = box.scrollTop;
      box.scrollTop = oldTop;
      return { before, attempted: box.scrollTop, activeTag: document.activeElement?.tagName, documentHasFocus: document.hasFocus() };
    }, oldTop);
    assert.equal(evidence.passiveRestoration.activeTag, 'BODY');
    assert(evidence.passiveRestoration.documentHasFocus, 'Composer blur keeps this document focused');
    assert(evidence.passiveRestoration.before - evidence.passiveRestoration.attempted > 80, 'The injected native restoration must actually displace the message viewport');
    evidence.blurRestored = await samples(page, selector, null, 1000);
    assertTail(evidence.blurRestored, 'A late passive restoration cannot replace the latest-message intent');

    const beforeReader = evidence.blurRestored.at(-1);
    const bounds = await page.locator(selector).boundingBox();
    await page.locator(selector).evaluate(box => {
      window.__qualityBlurWheel = null;
      box.addEventListener('wheel', event => { window.__qualityBlurWheel = { trusted: event.isTrusted, delta: event.deltaY }; }, { once: true, passive: true });
    });
    await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
    await page.mouse.wheel(0, -400);
    evidence.blurReader = await samples(page, selector, null, 800);
    evidence.blurWheel = await page.evaluate(() => window.__qualityBlurWheel);
    assert(evidence.blurWheel?.trusted, 'A real wheel takes reading ownership after composer blur');
    assert(evidence.blurReader.at(-1).top < beforeReader.top - 200, 'Composer ownership cannot undo a reader moving to history');

    await page.getByRole('button', { name: '返回', exact: true }).first().focus();
    evidence.otherControl = await page.evaluate(() => ({ tag: document.activeElement?.tagName, text: document.activeElement?.getAttribute('aria-label') }));
    evidence.ordinaryScroll = await page.locator(selector).evaluate(box => {
      const before = box.scrollTop;
      box.scrollTop = before - 80;
      return { before, chosen: box.scrollTop };
    });
    evidence.afterOtherFocus = await samples(page, selector, null, 650);
    assert(Math.abs(evidence.afterOtherFocus.at(-1).top - evidence.ordinaryScroll.chosen) <= 2, 'An explicit focus transfer preserves ordinary non-composer scrolling');
    return;
  }
  if (phase === 'tail') {
    await keyboard(page, 300, evidence, 'keyboardOpenFrames');
    evidence.keyboardOpen = await samples(page, selector);
    assertTail(evidence.keyboardOpenFrames, 'During keyboard opening');
    assertTail(evidence.keyboardOpen, 'Keyboard open');
    assert(evidence.keyboardOpen.at(-1).view < evidence.before.at(-1).view - 150, 'Keyboard must actually shrink the message viewport');
    await add(3);
    await frames(page);
    evidence.newMessage = await samples(page, selector);
    assertTail(evidence.newMessage, 'New messages with keyboard open');
    const inputBefore = await input.evaluate(el => el.getBoundingClientRect().height);
    await input.fill(Array.from({ length: 10 }, (_, i) => `保留草稿 ${i + 1}`).join('\n'));
    await page.waitForFunction(height => document.querySelector('[data-composer-text]').getBoundingClientRect().height > height + 10, inputBefore);
    await frames(page);
    evidence.composerGrowth = await samples(page, selector);
    assertTail(evidence.composerGrowth, 'Growing composer');
    await input.fill('');
    await keyboard(page, 0, evidence, 'keyboardCloseFrames');
    evidence.keyboardClose = await samples(page, selector);
    assertTail(evidence.keyboardCloseFrames, 'During keyboard closing');
    assertTail(evidence.keyboardClose, 'Keyboard closed');
    return;
  }
  // A real wheel gesture chooses history. Never assign scrollTop or refocus
  // the anchor; that would hide the defect under test.
  const bounds = await page.locator(selector).boundingBox();
  await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  await page.mouse.wheel(0, -800);
  await toBottom.waitFor({ state: 'visible' });
  evidence.historySettled = await samples(page, selector, null, 350);
  const anchor = await page.locator(selector).evaluate((box, kind) => {
    const rect = box.getBoundingClientRect();
    const node = [...box.querySelectorAll(kind === 'acp' ? 'p' : '[data-msg-text]')].find(el => el.textContent.startsWith(`滚动验收 ${kind}`) && el.getBoundingClientRect().top >= rect.top + 2 && el.getBoundingClientRect().bottom < rect.bottom);
    if (!node) throw new Error('Fixture must expose a complete historical paragraph');
    return { text: node.textContent, offset: node.getBoundingClientRect().top - rect.top };
  }, kind);
  evidence.anchor = anchor;
  if (phase === 'input-history') return inputDuringResize(page, selector, evidence);
  const anchorOffset = () => page.getByText(anchor.text, { exact: true }).evaluate((node, selector) => node.getBoundingClientRect().top - document.querySelector(selector).getBoundingClientRect().top, selector);
  await keyboard(page, 300, evidence, 'historyOpenFrames');
  evidence.historyOpen = await anchorOffset();
  assert(Math.abs(evidence.historyOpen - anchor.offset) <= 2, 'Keyboard opening keeps the historical paragraph anchored');
  // Reading history pauses body loading; the live content-change signal must
  // arrive without moving the anchor. The body is required on explicit return.
  const newest = await add(3, false);
  await frames(page, 5);
  evidence.historyMessage = await anchorOffset();
  assert(Math.abs(evidence.historyMessage - anchor.offset) <= 2, 'New messages do not take the reader away from history');
  await keyboard(page, 0, evidence, 'historyCloseFrames');
  evidence.historyClose = await anchorOffset();
  assert(Math.abs(evidence.historyClose - anchor.offset) <= 2, 'Keyboard closing keeps the historical paragraph anchored');
  assert(await toBottom.isVisible(), 'History mode remains an explicit choice');
  await toBottom.click();
  await page.getByText(newest, { exact: true }).waitFor({ state: 'attached' });
  await frames(page);
  evidence.returnToTail = await samples(page, selector);
  assertTail(evidence.returnToTail, 'Explicit return to the latest message');
}

async function lockCase(page, mode, evidence) {
  const facts = { on: true, has_pin: true, timeout_min: 0, timeouts: [0, 1, 5], pin_min: 4, system_door: false };
  let connected = mode !== 'unavailable', reads = 0, pinOutcome = 'offline';
  await page.route('**/lock_state', async route => {
    reads++;
    await route.fulfill(connected
      ? { contentType: 'application/json', body: JSON.stringify(facts) }
      : { status: 503, body: 'fixture configuration unavailable' });
  });
  await page.route('**/unlock', route => route.fulfill(pinOutcome === 'offline'
    ? { status: 503, body: 'fixture verification unavailable' }
    : { contentType: 'application/json', body: JSON.stringify(pinOutcome === 'accepted') }));
  await page.goto(`${base}/?${new URLSearchParams({ phone: '1', labSession: 'gui/ai-elements-review', labTabs: 'gui/ai-elements-review' })}`, {
    waitUntil: 'domcontentloaded', timeout: 45_000,
  });
  const hidden = async () => assert.equal(await page.locator('[data-chat-scroll], [data-machine-chat-scroll], [data-composer-text]').count(), 0, 'Locked content is removed from the document');
  if (mode === 'unavailable') {
    await page.locator('[data-lock-unavailable]').waitFor();
    await hidden();
    assert.equal(await page.locator('[data-lock-pin]').count(), 0, 'Unknown configuration cannot offer an authentication form');
    const before = reads, deadline = Date.now() + 4000;
    while (reads === before && Date.now() < deadline) await sleep(50);
    assert(reads > before, 'Failed configuration retries automatically');
    await hidden();
    connected = true;
    await page.locator('[data-lock-retry]').click();
    await page.locator('[data-lock-pin]').waitFor();
    assert.equal(await page.locator('[data-lock-unavailable]').count(), 0);
    await hidden();
  } else {
    const input = page.locator('[data-lock-pin]');
    await input.fill('1234');
    await page.locator('[data-lock-open]').click();
    await page.locator('[data-lock-verification-failed]').waitFor();
    assert.equal(await page.locator('[data-lock-wrong]').count(), 0, 'Transport failure is not a wrong PIN');
    await hidden();
    pinOutcome = 'wrong';
    await page.locator('[data-lock-open]').click();
    await page.locator('[data-lock-wrong]').waitFor();
    assert.equal(await page.locator('[data-lock-verification-failed]').count(), 0);
    await hidden();
  }
  pinOutcome = 'accepted';
  await page.locator('[data-lock-pin]').fill('1234');
  await page.locator('[data-lock-open]').click();
  await page.locator('[data-chat-scroll]').waitFor();
  assert.equal(await page.locator('[data-lock-screen]').count(), 0);
  evidence.configurationReads = reads;
  evidence.recovered = true;
}

try {
  await startPreview();
  for (const engine of engines) {
    browser = await (engine === 'chrome' ? chromium : webkit).launch({ ...(engine === 'chrome' ? { channel: (process.env.PLAYWRIGHT_CHROMIUM_CHANNEL ?? 'chrome') || undefined } : {}), headless });
    for (const name of names.filter(name => !only.length || only.some(filter => name === filter || name.startsWith(`${filter}-`)))) {
      const mobile = name.startsWith('lock-') || name === 'commit-touch' || name.startsWith('chat-');
      // Playwright cannot send a wheel in mobile WebKit. History keeps the
      // phone viewport/coarse pointer, with desktop WebKit wheel capability.
      const mobileEmulation = mobile && !(engine === 'webkit' && (name.endsWith('-history') || name.includes('-input-') || name.endsWith('-blur-restore')));
      const context = await browser.newContext({ viewport: mobile || name.startsWith('lab-') ? { width: 390, height: 844 } : { width: 1280, height: 900 }, hasTouch: mobile, isMobile: mobileEmulation, reducedMotion: 'no-preference' });
      const page = await context.newPage();
      page.setDefaultTimeout(12_000);
      const result = { engine, version: browser.version(), name, mobileEmulation, errors: [], evidence: {}, status: 'running' };
      report.results.push(result);
      page.on('pageerror', error => result.errors.push(error.message));
      try {
        if (name.startsWith('lock-')) await lockCase(page, name.slice(5), result.evidence);
        else if (name.startsWith('commit-')) await commitCase(page, name.slice(7), result.evidence);
        else if (name === 'lab-acp-blur') await labBlurCase(page, result.evidence);
        else await chatCase(page, name.split('-')[1], result.evidence, name.split('-').slice(2).join('-'));
        assert.deepEqual(result.errors, [], 'No uncaught browser errors');
        result.status = 'passed';
      } catch (error) {
        result.status = 'failed'; result.error = error.stack ?? String(error);
        process.exitCode = 1;
      } finally {
        await page.screenshot({ path: join(out, `${engine}-${name}.png`) }).catch(error => { result.screenshotError = String(error); });
        save();
        console.log(`${result.status.toUpperCase()} ${engine} ${name}${result.error ? `: ${result.error.split('\n')[0]}` : ''}`);
        await context.close();
      }
    }
    await browser.close(); browser = null;
  }
} catch (error) {
  report.error = error.stack ?? String(error); process.exitCode = 1;
  console.error(error);
} finally {
  await browser?.close(); await stopPreview();
  report.finished = new Date().toISOString();
  report.finalFiles = Object.fromEntries(files.map(file => [file, hash(readFileSync(join(gui, file)))]));
  report.changedDuringRun = files.filter(file => report.finalFiles[file] !== report.files[file]);
  if (report.changedDuringRun.length) {
    report.error = `Source changed during the run: ${report.changedDuringRun.join(', ')}`;
    process.exitCode = 1;
  }
  if (serverLog) writeFileSync(join(out, 'preview.log'), serverLog);
  save(); console.log(`Scroll lifecycle evidence: ${out}`);
}
