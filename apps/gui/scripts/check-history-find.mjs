import assert from 'node:assert/strict';
import { build } from 'esbuild';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { Worker as NodeWorker } from 'node:worker_threads';
const built = await build({ entryPoints: [resolve('src/lib/history-find-model.ts')], bundle: true,
  write: false, format: 'cjs', platform: 'node' });
const module = { exports: {} };
Function('module', 'exports', built.outputFiles[0].text)(module, module.exports);
const { historyFindBatch, literalMatcher, findQuery, FIND_SNIPPET_BYTES } = module.exports;
const projectionBuilt = await build({ entryPoints: [resolve('src/lib/acp-view.ts')], bundle: true,
  write: false, format: 'cjs', platform: 'node' });
const projectionModule = { exports: {} };
Function('module', 'exports', projectionBuilt.outputFiles[0].text)(projectionModule, projectionModule.exports);
const { toolInputText } = projectionModule.exports;
const scaffoldBuilt = await build({ entryPoints: [resolve('src/lib/history-record-scaffold.ts')], bundle: true,
  write: false, format: 'cjs', platform: 'node' });
const scaffoldModule = { exports: {} };
Function('module', 'exports', scaffoldBuilt.outputFiles[0].text)(scaffoldModule, scaffoldModule.exports);
const { recordScaffold, decodeRecordString } = scaffoldModule.exports;
const query = (text, caseSensitive = false) => ({ text, caseSensitive });
const note = (key, update) => ({ key, frame: { kind: 'note', update: { update } } });
const text = (key, value) => note(key, { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: value } });
const page = (entries, references = []) => ({ entries, references, older: 'older-source-cursor', newer: 'exclusive-source-fence', has_more: false,
  reset: false, encoded_bytes: 100 });
const checks = [];
function test(name, run) { checks.push([name, run]); }
test('uses the actual rendered projection, not fields or encoded attachment bytes', () => {
  const entries = [text('1:0', 'visible'), note('2:0', { sessionUpdate: 'agent_message_chunk',
    content: { type: 'image', data: 'HIDDEN_NEEDLE', mimeType: 'image/png' }, HIDDEN_NEEDLE: 'not body' })];
  assert.equal(historyFindBatch(query('HIDDEN_NEEDLE')).page(page(entries)).hits.length, 0);
  const hit = historyFindBatch(query('VISIBLE')).page(page(entries)).hits[0];
  assert.equal(hit.entry, '1:0');
  assert.equal(hit.before, 'exclusive-source-fence');
  assert.deepEqual([hit.from, hit.to], [0, 7]);
});
test('streams Unicode matching across chunks with exact original offsets', () => {
  const matcher = literalMatcher(query('中文😀x'));
  assert.deepEqual(matcher.feed('head中').ranges, []);
  assert.deepEqual(matcher.feed('文😀').ranges, []);
  assert.deepEqual(matcher.feed('Xtail').ranges, [{ from: 4, to: 9 }]);
  const expanded = literalMatcher(query('i'));
  assert.deepEqual(expanded.feed('İ').ranges, [{ from: 0, to: 1 }]);
  assert.deepEqual(literalMatcher(query('ΟΣ')).feed('ος').ranges, [{ from: 0, to: 2 }]);
  assert.equal(historyFindBatch(query('VISIBLE', true)).page(page([text('1:0', 'visible')])).hits.length, 0);
});
test('a thirty-two-hit batch resumes inside its record without dropped matches', () => {
  const scan = historyFindBatch(query('match'));
  const first = scan.page(page([text('1:0', 'match '.repeat(70))]));
  const second = scan.next();
  const third = scan.next();
  assert.deepEqual([first.hits.length, second.hits.length, third.hits.length], [32, 32, 6]);
  const offsets = [...first.hits, ...second.hits, ...third.hits].map(hit => hit.from);
  assert.deepEqual(offsets, Array.from({ length: 70 }, (_, i) => i * 6));
  assert.equal(third.pageDone, true);
});
test('result bytes can stop before thirty-two hits without losing the rejected match', () => {
  const scan = historyFindBatch(query('a'));
  const before = '\\'.repeat(4000);
  let result = scan.page({ ...page([text('1:0', 'a '.repeat(70))]), newer: before });
  const found = [];
  for (let batches = 0; ; batches++) {
    assert.equal(batches < 70, true);
    assert.equal(new TextEncoder().encode(JSON.stringify(result)).length <= 64 * 1024, true);
    found.push(...result.hits.map(hit => hit.from));
    if (result.pageDone) break;
    result = scan.next();
  }
  assert.deepEqual(found, Array.from({ length: 70 }, (_, i) => i * 2));
});
test('tool updates and rich text use the existing display projection', () => {
  const entries = [note('1:0', { sessionUpdate: 'tool_call', toolCallId: 'call', title: 'Edit',
    content: [{ type: 'content', content: { type: 'text', text: 'old output' } }] }),
  note('2:0', { sessionUpdate: 'tool_call_update', toolCallId: 'call',
    content: [{ type: 'diff', path: 'a.txt', oldText: '原来', newText: '现在的正文' }] })];
  assert.equal(historyFindBatch(query('old output')).page(page(entries)).hits.length, 0);
  assert.equal(historyFindBatch(query('现在的正文')).page(page(entries)).hits.length, 1);
  assert.equal(historyFindBatch(query('newText')).page(page(entries)).hits.length, 0);
});
test('retains bounded UTF-8 snippets and explicitly reports unresolved raw ranges', () => {
  const answer = historyFindBatch(query('needle')).page(page([text('1:0', '😀'.repeat(90) + 'needle' + '中'.repeat(90))], [{ cursor: 'raw' }]));
  assert.equal(new TextEncoder().encode(answer.hits[0].snippet).length <= FIND_SNIPPET_BYTES, true);
  assert.equal(answer.hits[0].snippet.includes('\ufffd'), false);
  assert.equal(answer.unsearchedRanges, 1);
});
test('query and page budgets reject invalid work without losing a pending page', () => {
  assert.throws(() => findQuery(query('')));
  assert.throws(() => findQuery(query('a'.repeat(257))));
  assert.throws(() => findQuery(query('\ud800')));
  const scan = historyFindBatch(query('a'));
  assert.throws(() => scan.page({ ...page([]), encoded_bytes: 512 * 1024 + 1 }));
  assert.throws(() => scan.page(page([text('x'.repeat(300000), 'a')])));
  const accepted = scan.page(page([text('1:0', 'a'.repeat(40))]));
  assert.equal(accepted.hits.length, 32);
  assert.throws(() => scan.page(page([])));
  assert.equal(scan.next().hits.length, 8);
});
test('tool input render/copy/search share text and oversized pretty input is rejected before constructing it', () => {
  const raw = { path: '中文', values: [null, true, '😀\n\u0000', { value: 42 }] };
  const rendered = toolInputText(raw);
  assert.equal(rendered.text, JSON.stringify(raw, null, 2));
  const bytes = new TextEncoder().encode(rendered.text).length;
  assert.deepEqual(toolInputText(raw, bytes), rendered);
  assert.equal(toolInputText(raw, bytes - 1).complete, false);
  assert.deepEqual(toolInputText({ command: 'echo NEEDLE' }, 64 * 1024), { text: 'echo NEEDLE', command: true, complete: true });
  const tool = note('1:0', { sessionUpdate: 'tool_call', toolCallId: 'call', title: 'Run', rawInput: { command: 'echo NEEDLE' } });
  assert.equal(historyFindBatch(query('needle')).page(page([tool])).hits.length, 1);
  const stringify = JSON.stringify;
  let constructed = 0;
  JSON.stringify = (...args) => { constructed++; return stringify(...args); };
  try {
    const oversized = toolInputText({ body: 'x'.repeat(70 * 1024) }, 64 * 1024);
    assert.equal(oversized.complete, false);
    assert.equal(oversized.text, undefined);
    assert.equal(constructed, 0);
  } finally { JSON.stringify = stringify; }
  const deep = { value: 'x'.repeat(30000) };
  let nested = deep;
  for (let i = 0; i < 90; i++) nested = { nested, pad: ' '.repeat(150) };
  assert.equal(new TextEncoder().encode(JSON.stringify(nested)).length < 64 * 1024, true, 'fixture compact input remains inline-sized');
  assert.equal(new TextEncoder().encode(JSON.stringify(nested, null, 2)).length > 64 * 1024, true, 'fixture pretty body actually exceeds its budget');
  const answer = historyFindBatch(query('absent')).page(page([note('2:0', { sessionUpdate: 'tool_call', toolCallId: 'large', title: 'Call', rawInput: nested })]));
  assert.equal(answer.unsearchedRanges, 1);
});
test('pure projection releases the unused home admission once, including failed transport', async () => {
  const calls = [];
  const projected = { entries: [], complete: false, record: { cursor: 'joined' } };
  const source = historySource('binding', {
    begin: async () => 'projection-lease',
    cancel: async key => { calls.push(['cancel', key]); },
    project: async (...args) => { calls.push(['project', ...args]); return projected; },
    page: async () => { throw new Error('projection must not scan a page'); },
    record: async () => { throw new Error('projection must not read original bytes'); },
  });
  assert.equal(await source.project('fragment', 1, 100, '{}'), projected);
  assert.deepEqual(calls.map(call => call[0]), ['project', 'cancel']);
  assert.equal(calls[0][1], 'binding');
  assert.equal(calls[0][2], 'projection-lease');
  assert.deepEqual(await source.dispose(), { failures: 0, lastError: null });
  const failed = [];
  const failure = new Error('projection refused');
  const other = historySource('binding', {
    begin: async () => 'failed-projection', cancel: async key => { failed.push(key); },
    project: async () => { throw failure; },
    page: async () => { throw new Error('unexpected page'); }, record: async () => { throw new Error('unexpected raw'); },
  });
  await assert.rejects(other.project('fragment', 1, 100, null), error => error === failure);
  assert.deepEqual(failed, ['failed-projection']);
  await other.dispose();
  const heldAck = deferred(), cancelling = deferred();
  const controller = new AbortController();
  let cancels = 0;
  const held = historySource('binding', {
    begin: async () => 'held-projection',
    cancel: async () => { cancels++; cancelling.resolve(); await heldAck.promise; },
    project: async () => projected,
    page: async () => { throw new Error('unexpected page'); }, record: async () => { throw new Error('unexpected raw'); },
  });
  const heldJob = held.project('fragment', 1, 100, '{}', controller.signal);
  await cancelling.promise;
  controller.abort();
  const disposed = held.dispose();
  heldAck.resolve();
  assert.equal(await heldJob, null, 'retirement during release ACK must reject an already-computed projection');
  await disposed;
  assert.equal(cancels, 1);
  const entered = deferred(), heldWork = deferred();
  let earlyCancels = 0;
  const early = historySource('binding', {
    begin: async () => 'early-projection', cancel: async () => { earlyCancels++; },
    project: async () => { entered.resolve(); return heldWork.promise; },
    page: async () => { throw new Error('unexpected page'); }, record: async () => { throw new Error('unexpected raw'); },
  });
  const earlyJob = early.project('fragment', 1, 100, '{}');
  await entered.promise;
  const earlyDisposed = early.dispose();
  await new Promise(resolve => setImmediate(resolve));
  heldWork.resolve(projected);
  assert.equal(await earlyJob, null);
  await earlyDisposed;
  assert.equal(earlyCancels, 1, 'a completed early release must not be repeated when work answers');
});
test('giant record preparation streams prefixes and exact raw ranges despite field order and split Unicode', () => {
  const body = '<system-reminder>' + '中😀'.repeat(25000) + 'NEEDLE';
  const binary = 'A'.repeat(180000);
  const file = '\t路径\n'.repeat(13000);
  // Discriminators follow the large values. The scanner preserves structure,
  // not a guessed provider/field order, and retains no complete large string.
  const raw = JSON.stringify({ files: [['a.txt', file]], images: [['image/png', binary]],
    text: body, turn: 'local-turn', kind: 'prompt' }) + '\n';
  const bytes = new TextEncoder().encode(raw);
  const parser = recordScaffold(17, 'fixture');
  const widths = [1, 2, 3, 7, 4093, 65536];
  for (let at = 0, step = 0; at < bytes.length; step++) {
    const end = Math.min(bytes.length, at + widths[step % widths.length]);
    parser.push(bytes.subarray(at, end), 17 + at); at = end;
  }
  const prepared = parser.finish(17 + bytes.length);
  assert.equal(new TextEncoder().encode(prepared.json).length <= 64 * 1024, true);
  assert.equal(prepared.json.length < 2000, true);
  const skeleton = JSON.parse(prepared.json);
  assert.equal(skeleton.kind, 'prompt');
  assert.equal(skeleton.turn, 'local-turn');
  assert.equal(skeleton.text.startsWith('<system-reminder>'), true);
  assert.equal(prepared.json.includes('NEEDLE'), false);
  assert.equal(prepared.strings.length, 3);
  const originals = prepared.strings.map(range => JSON.parse(new TextDecoder().decode(bytes.subarray(range.start - 17, range.end - 17))));
  assert.deepEqual(originals, [file, binary, body]);
  assert.equal(prepared.strings.every(range => Object.keys(range).sort().join(',') === 'end,marker,start'), true);
  assert.throws(() => parser.push(new Uint8Array(), 17 + bytes.length), /retired/);
});
test('the record lexer validates escape and grammar boundaries and fails budgets explicitly', () => {
  for (const raw of [String.raw`{"value":"中😀\ud83d\ude03\n\t\\\"","n":-1.25e+2,"other":[true,false,null,{},[]]}`,
    ' {"duplicate":1,"duplicate":2} ', '"plain"', '[0,-0,1e2]']) {
    const bytes = new TextEncoder().encode(raw), parser = recordScaffold(0, 'small');
    for (let at = 0; at < bytes.length; at++) parser.push(bytes.subarray(at, at + 1), at);
    assert.deepEqual(JSON.parse(parser.finish(bytes.length).json), JSON.parse(raw));
  }
  for (const raw of ['[1,]', '{"a":1,}', '{"a" 1}', '{"a":01}', 'true false', String.raw`"\x"`, String.raw`"\ud800"`, '{']) {
    const bytes = new TextEncoder().encode(raw), parser = recordScaffold(0, 'bad');
    assert.throws(() => { parser.push(bytes, 0); parser.finish(bytes.length); });
  }
  const incomplete = recordScaffold(0, 'utf8');
  const bytes = Uint8Array.from([0x22, 0xe4, 0xb8]);
  incomplete.push(bytes, 0);
  assert.throws(() => incomplete.finish(bytes.length));
  const discontinuous = recordScaffold(1, 'gap');
  assert.throws(() => discontinuous.push(new Uint8Array([0x7b]), 2), /contiguous/);
  const oversized = recordScaffold(0, 'chunk');
  assert.throws(() => oversized.push(new Uint8Array(65537), 0), /chunk.*budget/);
  const many = new TextEncoder().encode(JSON.stringify(Array.from({ length: 129 }, () => 'x'.repeat(1025))));
  const limited = recordScaffold(0, 'ranges');
  assert.throws(() => { for (let at = 0; at < many.length; at += 65536) limited.push(many.subarray(at, at + 65536), at); limited.finish(many.length); }, /too many/);
  const structure = new TextEncoder().encode(JSON.stringify(Object.fromEntries(Array.from({ length: 8000 }, (_, i) => [`key${i}`, 0]))));
  const metadata = recordScaffold(0, 'structure');
  assert.throws(() => { for (let at = 0; at < structure.length; at += 65536) metadata.push(structure.subarray(at, at + 65536), at); metadata.finish(structure.length); }, /scaffold budget/);
  const key = new TextEncoder().encode(JSON.stringify({ ['\u0000'.repeat(12000)]: 1 }));
  const keys = recordScaffold(0, 'keys');
  const stringify = JSON.stringify;
  let constructed = 0;
  JSON.stringify = (...args) => { constructed++; return stringify(...args); };
  try {
    assert.throws(() => { for (let at = 0; at < key.length; at += 65536) keys.push(key.subarray(at, at + 65536), at); }, /key.*scaffold budget/);
    assert.equal(constructed, 0, 'escaped key budget must fail before constructing its JSON string');
  } finally { JSON.stringify = stringify; }
});
test('the second pass shares that lexer and pauses a decoded match without buffering its remaining body', () => {
  const raw = String.raw`"前\ud83d\ude03\nNEEDLE中😀tail"`;
  const bytes = new TextEncoder().encode(raw);
  const emitted = [];
  let pause = true;
  const decoder = decodeRecordString(100, (char, from, to, start, end) => {
    emitted.push({ char, from, to, start, end });
    if (pause && char === 'N') { pause = false; return false; }
    return true;
  });
  decoder.push(bytes, 100);
  assert.equal(decoder.pending(), true);
  assert.equal(emitted.map(atom => atom.char).join(''), '前😃\nN');
  assert.throws(() => decoder.push(new Uint8Array(), 100 + bytes.length), /not been consumed/);
  // The illegal extra push retires that reader. Re-run a legitimate pause
  // to prove resume separately, without accepting a poisoned decoder.
  const matching = literalMatcher(query('😃\nneedle'));
  const matches = [];
  const second = decodeRecordString(100, (char, _from, _to, start, end) => {
    for (const range of matching.feed(char, 1).ranges) matches.push({ ...range, lastRawStart: start, lastRawEnd: end });
    return false;
  });
  second.push(bytes, 100);
  while (second.pending()) second.resume();
  second.finish(100 + bytes.length);
  assert.equal(matches.length, 1);
  assert.deepEqual([matches[0].from, matches[0].to], [1, 10]);
  const unicode = emitted.find(atom => atom.char === '😃');
  assert.equal(new TextDecoder().decode(bytes.subarray(unicode.start - 100, unicode.end - 100)), String.raw`\ud83d\ude03`);
  const notString = decodeRecordString(0, () => true);
  assert.throws(() => notString.push(new TextEncoder().encode('{}'), 0), /not a JSON string/);
});
const deferred = () => {
  let resolve;
  const promise = new Promise(yes => { resolve = yes; });
  return { promise, resolve };
};
const workerBuilt = await build({ entryPoints: [resolve('src/lib/history-find.worker.ts')], bundle: true,
  write: false, format: 'cjs', platform: 'node' });
const controllerBuilt = await build({ entryPoints: [resolve('src/lib/history-find.ts')], bundle: true,
  write: false, format: 'cjs', platform: 'node', define: { 'import.meta.url': JSON.stringify(pathToFileURL(resolve('src/lib/history-find.ts')).href) } });
const controllerModule = { exports: {} };
Function('module', 'exports', controllerBuilt.outputFiles[0].text)(controllerModule, controllerModule.exports);
const sourceBuilt = await build({ entryPoints: [resolve('src/lib/history-source.ts')], bundle: true,
  write: false, format: 'cjs', platform: 'node' });
const sourceModule = { exports: {} };
Function('module', 'exports', sourceBuilt.outputFiles[0].text)(sourceModule, sourceModule.exports);
const { historyFind } = controllerModule.exports;
const { historySource } = sourceModule.exports;
const ownedWorkers = new Set();
const workerExits = [];
const OriginalWorker = globalThis.Worker;
class BrowserWorker {
  constructor() {
    // Browser transport adapter only: the actual production worker, matcher,
    // and projection execute in a separate Node worker thread unchanged.
    this.worker = new NodeWorker(`const { parentPort } = require('node:worker_threads');
      globalThis.self = { postMessage: value => parentPort.postMessage(value) };
      parentPort.on('message', data => self.onmessage?.({ data }));
      ${workerBuilt.outputFiles[0].text}`, { eval: true });
    ownedWorkers.add(this.worker);
    workerExits.push(new Promise(resolve => this.worker.once('exit', () => { ownedWorkers.delete(this.worker); resolve(); })));
    this.worker.on('message', data => this.onmessage?.({ data }));
    this.worker.on('error', error => this.onerror?.({ message: String(error) }));
  }
  postMessage(message) { this.worker.postMessage(message); }
  terminate() { void this.worker.terminate(); }
}
globalThis.Worker = BrowserWorker;
test('actual worker resumes a source page and disposal cancels its old read without revival', async () => {
  const waiting = deferred();
  const entered = deferred();
  const results = [];
  const cancelled = [];
  let leases = 0;
  const source = historySource('binding', {
    begin: async () => `lease${++leases}`,
    cancel: async key => { cancelled.push(key); },
    page: async (_binding, key, before) => {
      if (before === null) return page([text('1:0', 'needle '.repeat(70))]);
      assert.equal(before, 'older-source-cursor');
      entered.resolve(key);
      return waiting.promise;
    },
    record: () => { throw new Error('normal search must not expand raw records'); },
  });
  let arrived = deferred();
  const reader = historyFind(source, query('needle'), state => {
    results.push(state);
    if (!state.busy) arrived.resolve(state);
  });
  try {
    reader.start();
    assert.equal((await arrived.promise).hits.length, 32);
    arrived = deferred(); reader.next();
    assert.equal((await arrived.promise).hits.length, 32);
    arrived = deferred(); reader.next();
    const third = await arrived.promise;
    assert.equal(third.hits.length, 6);
    assert.equal(third.scannedPages, 1);
    reader.next();
    const lease = await entered.promise;
    const cleanup = reader.dispose();
    const published = results.length;
    waiting.resolve({ ...page([text('stale:0', 'needle')]), older: null });
    assert.deepEqual(await cleanup, { failures: 0, lastError: null });
    assert.equal(cancelled.includes(lease), true);
    assert.equal(results.length, published);
  } finally {
    waiting.resolve({ ...page([]), older: null });
    await reader.dispose();
  }
});
test('source replacement stops the real worker and never presents the new page as old matches', async () => {
  const arrived = deferred();
  const source = historySource('binding', {
    begin: async () => 'replacement-lease', cancel: async () => {},
    page: async () => ({ ...page([text('new:0', 'needle')]), reset: true }),
    record: () => { throw new Error('unexpected raw read'); },
  });
  const reader = historyFind(source, query('needle'), state => { if (state.reset) arrived.resolve(state); });
  try {
    reader.start();
    const reset = await arrived.promise;
    assert.equal(reset.reset, true);
    assert.equal(reset.complete, false);
    assert.deepEqual(reset.hits, []);
  } finally { await reader.dispose(); }
  const invalid = deferred();
  let last;
  const stopped = historyFind(historySource('binding', {
    begin: async () => { throw new Error('invalid query must never begin IO'); },
    cancel: async () => {}, page: async () => { throw new Error('unexpected page IO'); },
    record: async () => { throw new Error('unexpected raw IO'); },
  }), query('x'.repeat(257)), state => { last = state; if (state.error) invalid.resolve(); });
  try {
    await invalid.promise;
    await stopped.start();
    assert.equal(last.busy, false);
  } finally { await stopped.dispose(); }
});
try {
  await Promise.race([
    (async () => { for (const [name, run] of checks) { await run(); console.log('PASS', name); } })(),
    new Promise((_, reject) => { const timer = setTimeout(() => reject(new Error('history find fixture exceeded 10s')), 10_000); timer.unref(); }),
  ]);
} finally {
  for (const worker of ownedWorkers) await worker.terminate();
  await Promise.all(workerExits);
  globalThis.Worker = OriginalWorker;
}
assert.equal(ownedWorkers.size, 0);
console.log(`${checks.length} source-backed history find checks passed; actual worker threads, no browser/raw-record projection claim`);
