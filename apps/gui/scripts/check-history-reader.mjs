import assert from 'node:assert/strict';
import { build } from 'esbuild';
import { resolve } from 'node:path';
import { readFileSync } from 'node:fs';
import ts from 'typescript';
const built = await build({ stdin: { contents: ['history-window', 'history-source', 'history-reader', 'history-record', 'history-paint'].map(name => `export * from './${name}';`).join('\n'), resolveDir: resolve('src/lib'), loader: 'ts' }, bundle: true, write: false, format: 'cjs', platform: 'node' });
const module = { exports: {} };
Function('module', 'exports', built.outputFiles[0].text)(module, module.exports);
const { EMPTY_HISTORY_WINDOW: EMPTY, receiveHistoryPage: receive, historyEntries, historySource, historyReader, decodeRecordSection, paintHistory } = module.exports;
const note = (sessionUpdate, extra = {}) => ({ kind: 'note', update: { update: { sessionUpdate, ...extra } } });
const entry = (key, text = key) => ({ key, frame: note('agent_message_chunk', { content: { type: 'text', text } }) });
const page = (key, { count = 1, bytes = 100, older = `before:${key}`, newer = `after:${key}`, more = false, reset = false, refs = [] } = {}) => ({ entries: Array.from({ length: count }, (_, i) => entry(`${key}:${i}`)), older, newer, has_more: more, reset, references: refs, encoded_bytes: bytes });
const deferred = () => { let resolve, reject; const promise = new Promise((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; };
const flush = () => new Promise(resolve => setImmediate(resolve));
const checks = [];
const test = (name, run) => checks.push([name, run]);
test('page eviction retains reload endpoints', () => {
  let w = receive(EMPTY, page('9'), 'tail');
  for (let i = 8; i >= 0; i--)
    w = receive(w, page(String(i)), 'older');
  assert.equal(w.pages.length, 8);
  assert.equal(w.older, 'before:0');
  assert.equal(w.newer, 'after:7');
  assert.equal(w.latest, false);
  w = receive(w, page('8'), 'newer');
  assert.equal(w.older, 'before:1');
});
test('frame and byte caps independently evict', () => {
  let w = EMPTY;
  for (let i = 0; i < 5; i++)
    w = receive(w, page(String(i), { count: 128 }), i ? 'newer' : 'tail');
  assert.equal(historyEntries(w).length, 512);
  w = EMPTY;
  for (let i = 0; i < 5; i++)
    w = receive(w, page(String(i), { bytes: 512 * 1024 }), i ? 'newer' : 'tail');
  assert.equal(w.pages.length, 4);
  assert.equal(w.pages.reduce((n, p) => n + p.encoded_bytes, 0), 2 * 1024 * 1024);
  assert.throws(() => receive(w, page('bad', { bytes: 512 * 1024 + 1 }), 'newer'));
});
test('empty older progress survives newer page', () => {
  let w = receive(EMPTY, page('base'), 'tail');
  w = receive(w, page('scan', { count: 0, older: null }), 'older');
  w = receive(w, page('append'), 'newer');
  assert.equal(w.older, null);
});
test('empty newer progress survives older page', () => {
  let w = receive(EMPTY, page('base'), 'tail');
  w = receive(w, page('scan', { count: 0, newer: 'advanced' }), 'newer');
  w = receive(w, page('old'), 'older');
  assert.equal(w.newer, 'advanced');
});
test('reset clears old-generation keys and references', () => {
  let w = receive(EMPTY, page('same', { refs: [{ start: 0, end: 5, item: null, cursor: 'old' }] }), 'tail');
  w = receive(w, page('same', { reset: true }), 'newer');
  assert.equal(w.pages.length, 1);
  assert.deepEqual(w.pages[0].references, []);
  assert.equal(historyEntries(w).length, 1);
});
test('growing unfinished reference replaces prior page', () => {
  const ref = end => ({ start: 0, end, item: null, missing_start: false, missing_end: true, cursor: String(end) });
  let w = receive(EMPTY, page('raw', { count: 0, refs: [ref(10)] }), 'tail');
  for (let end = 20; end < 1000; end += 10)
    w = receive(w, page('raw', { count: 0, refs: [ref(end)] }), 'newer');
  assert.equal(w.pages.length, 1);
  assert.equal(w.pages[0].references[0].end, 990);
  assert.equal(w.older, 'before:raw');
});
test('dispose cancels pending begin without dispatching IO', async () => {
  const begin = deferred(), calls = [];
  const s = historySource('s', { begin: () => begin.promise, cancel: async (key) => calls.push(['cancel', key]), page: async () => { calls.push(['page']); return page('x'); } });
  const job = s.page(null), disposed = s.dispose();
  begin.resolve('lease');
  assert.equal(await job, null);
  assert.deepEqual(await disposed, { failures: 0, lastError: null });
  assert.deepEqual(calls, [['cancel', 'lease']]);
});
test('transport failure cancels lease and preserves original error', async () => {
  const calls = [], failure = new Error('transport');
  const s = historySource('s', { begin: async () => 'key', cancel: async (key) => calls.push(key), page: async () => { throw failure; } });
  await assert.rejects(s.page(null), error => error === failure);
  assert.deepEqual(calls, ['key']);
  assert.equal((await s.dispose()).failures, 0);
});
test('dispose waits for abort cancellation with bounded failure summary', async () => {
  const response = deferred(), cancel = deferred(), controller = new AbortController();
  let cancels = 0;
  const s = historySource('s', { begin: async () => 'key', cancel: () => { cancels++; return cancel.promise; }, page: () => response.promise });
  const job = s.page(null, false, controller.signal);
  await flush();
  controller.abort();
  await flush();
  response.resolve(page('x'));
  assert.equal(await job, null);
  let finished = false;
  const disposal = s.dispose().then(result => { finished = true; return result; });
  await flush();
  assert.equal(finished, false);
  cancel.reject(new Error('x'.repeat(1000)));
  const result = await disposal;
  assert.equal(cancels, 1);
  assert.equal(result.failures, 1);
  assert.equal(result.lastError.length, 512);
});
function fixture(visible = true) {
  let canFollow = visible, callback = null;
  const calls = [], changes = [], responses = [];
  const source = historySource('s', { begin: async () => `lease:${calls.length}`, cancel: async (key) => calls.push(['cancel', key]), page: (id, key, cursor, newer) => { calls.push(['page', cursor, newer]); const response = deferred(); responses.push(response); return response.promise; } });
  const reader = historyReader(source, state => changes.push(state), () => canFollow, { schedule: run => { callback = run; return 1; }, cancel: () => { callback = null; } });
  return { reader, calls, changes, responses, visible: value => { canFollow = value; }, tick: () => { const run = callback; callback = null; run?.(); } };
}
test('hidden start does no IO; visible reader keeps one page active', async () => {
  const f = fixture(false);
  f.reader.start();
  await flush();
  assert.equal(f.calls.length, 0);
  f.visible(true);
  f.tick();
  await flush();
  assert.equal(f.responses.length, 1);
  f.reader.resume();
  f.reader.newer();
  await flush();
  assert.equal(f.responses.length, 1);
  f.responses[0].resolve(page('tail'));
  await flush();
  await f.reader.dispose();
});
test('away pauses follow; latest rejects a late prior response', async () => {
  const f = fixture();
  void f.reader.start();
  await flush();
  f.responses[0].resolve(page('tail'));
  await flush();
  f.visible(false);
  f.tick();
  await flush();
  assert.equal(f.responses.length, 1);
  f.visible(true);
  f.tick();
  await flush();
  assert.equal(f.responses.length, 2);
  void f.reader.latest();
  await flush();
  assert.equal(f.responses.length, 3);
  f.responses[2].resolve(page('fresh'));
  await flush();
  f.responses[1].resolve(page('stale'));
  await flush();
  assert.equal(historyEntries(f.changes.at(-1).window)[0].key, 'fresh:0');
  assert(f.calls.some(call => call[0] === 'cancel'));
  await f.reader.dispose();
});
test('automatic follow catches up through multiple bounded batches', async () => {
  const f = fixture();
  void f.reader.start();
  await flush();
  f.responses[0].resolve(page('tail'));
  await flush();
  f.tick();
  await flush();
  f.responses[1].resolve(page('backlog', {more: true}));
  await flush();
  assert.equal(f.changes.at(-1).window.latest, false);
  f.tick();
  await flush();
  assert.equal(f.responses.length, 3);
  assert.deepEqual(f.calls.filter(call => call[0] === 'page').at(-1), ['page', 'after:backlog', true]);
  f.responses[2].resolve(page('caught-up'));
  await flush();
  assert.equal(f.changes.at(-1).window.latest, true);
  await f.reader.dispose();
});
test('cursor-only polls preserve the content identity used by the painter', async () => {
  const f = fixture();
  void f.reader.start();
  await flush();
  f.responses[0].resolve(page('tail'));
  await flush();
  const pages = f.changes.at(-1).window.pages;
  f.tick();
  await flush();
  f.responses[1].resolve(page('scan', {count: 0, newer: 'advanced'}));
  await flush();
  assert.equal(f.changes.at(-1).window.pages, pages);
  assert.equal(f.changes.at(-1).window.newer, 'advanced');
  await f.reader.dispose();
});
test('pause and exit retire worker and reject late paint', async () => {
  const f = fixture();
  void f.reader.start();
  await flush();
  f.reader.pause();
  const count = f.changes.length;
  const disposed = f.reader.dispose();
  f.responses[0].resolve(page('late'));
  await disposed;
  assert.equal(f.changes.length, count);
  assert(f.calls.some(call => call[0] === 'cancel'));
});
test('streaming section decoder preserves split Chinese and emoji', () => {
  const original = 'a'.repeat(65535) + '中文🙂end', bytes = new TextEncoder().encode(original), decoder = new TextDecoder(), sections = [];
  for (let offset = 0; offset < bytes.length; offset += 65536) {
    const part = bytes.slice(offset, offset + 65536), next = offset + part.length < bytes.length ? offset + part.length : null;
    sections.push(decodeRecordSection({ offset, data: Buffer.from(part).toString('base64'), next }, decoder));
  }
  assert.equal(sections.join(''), original);
  assert(!sections.join('').includes('\ufffd'));
  assert(sections.every(section => section.length <= 65536));
});
test('switching raw reference then failing cannot reuse the previous offset', async () => {
  const path = 'src/components/history/HistoryRecords.tsx';
  const file = ts.createSourceFile(path, readFileSync(path, 'utf8'), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  let declaration;
  const visit = node => {
    if (ts.isVariableDeclaration(node) && node.name.getText(file) === 'load') declaration = node;
    ts.forEachChild(node, visit);
  };
  visit(file);
  assert(declaration?.initializer, 'real component load callback exists');
  const compiled = ts.transpile(`const load = ${declaration.initializer.getText(file)};`, {target: ts.ScriptTarget.ES2022});
  const state = {};
  const callbacks = {
    pending: {current: null}, decoder: {current: new TextDecoder()}, decodeRecordSection,
    setText: value => {state.text = value;}, setNext: value => {state.next = value;},
    setSelected: value => {state.selected = value;}, setBusy: value => {state.busy = value;},
    setError: value => {state.error = value;},
    read: async cursor => {
      if (cursor === 'failed') throw new Error('unreadable');
      return {data: Buffer.from('part').toString('base64'), next: 65536, offset: 0};
    },
  };
  const load = Function(...Object.keys(callbacks), compiled + '\nreturn load;')(...Object.values(callbacks));
  await load({cursor: 'first', start: 0}, 0, true);
  assert.equal(state.next, 65536);
  await load({cursor: 'failed', start: 42}, 42, true);
  assert.equal(state.selected.cursor, 'failed');
  assert.equal(state.next, null);
  assert.equal(state.busy, false);
});
test('prepend preserves paragraph keys and does not concatenate records', () => {
  const recent = [entry('20:0', 'second'), entry('30:0', 'third')], before = paintHistory(recent), after = paintHistory([entry('10:0', 'first'), ...recent]);
  assert.deepEqual(after.slice(1), before);
  assert.equal(after.length, 3);
  assert.equal(after[1].text, 'second');
});
test('orphan tool result displays and reconciles with an older call', () => {
  const call = { key: '0:0', frame: note('tool_call', { toolCallId: 'tool', title: 'Read', status: 'in_progress' }) }, result = { key: '10:0', frame: note('tool_call_update', { toolCallId: 'tool', status: 'completed', rawOutput: 'done' }) };
  const orphan = paintHistory([result]);
  assert.equal(orphan.length, 1);
  assert.equal(orphan[0].out, 'done');
  const joined = paintHistory([call, result]);
  assert.equal(joined.length, 1);
  assert.equal(joined[0].historyKey, '0:0:0');
  assert.equal(joined[0].status, 'completed');
  assert.equal(joined[0].out, 'done');
});
let failures = 0;
for (const [name, run] of checks) {
  try {
    await run();
    console.log('PASS', name);
  }
  catch (error) {
    failures++;
    console.error('FAIL', name, error);
  }
}
console.log(`${checks.length - failures}/${checks.length} history checks passed`);
assert.equal(failures, 0);
