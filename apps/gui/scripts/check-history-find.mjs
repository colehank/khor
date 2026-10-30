import assert from 'node:assert/strict';
import { build } from 'esbuild';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { Worker as NodeWorker } from 'node:worker_threads';
import { readFile } from 'node:fs/promises';
const args = process.argv.slice(2);
const only = args.includes('--only') ? args[args.indexOf('--only') + 1] : null;
const savedRecord = args.includes('--record-source') ? args[args.indexOf('--record-source') + 1] : null;
const savedPlugin = savedRecord ? [{ name: 'exact-saved-record-source', setup(build) {
  build.onLoad({ filter: /history-find-record\.ts$/ }, async () => ({ contents: await readFile(savedRecord, 'utf8'), loader: 'ts', resolveDir: resolve('src/lib') }));
} }] : [];
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
const recordBuilt = await build({ entryPoints: [resolve('src/lib/history-find-record.ts')], plugins: savedPlugin, bundle: true,
  write: false, format: 'cjs', platform: 'node' });
const recordModule = { exports: {} };
Function('module', 'exports', recordBuilt.outputFiles[0].text)(recordModule, recordModule.exports);
const { findRecord } = recordModule.exports;
const recordTextBuilt = await build({ entryPoints: [resolve('src/lib/history-record-text.ts')], bundle: true,
  write: false, format: 'cjs', platform: 'node' });
const recordTextModule = { exports: {} };
Function('module', 'exports', recordTextBuilt.outputFiles[0].text)(recordTextModule, recordTextModule.exports);
const { limitedToolOutput } = recordTextModule.exports;
const actionBuilt = await build({ entryPoints: [resolve('src/lib/history-find-action.ts')], bundle: true,
  write: false, format: 'cjs', platform: 'node' });
const actionModule = { exports: {} };
Function('module', 'exports', actionBuilt.outputFiles[0].text)(actionModule, actionModule.exports);
const { historyFindAction } = actionModule.exports;
const exportBuilt = await build({ entryPoints: [resolve('src/lib/history-export.ts')], bundle: true,
  write: false, format: 'cjs', platform: 'node' });
const exportModule = { exports: {} };
Function('module', 'exports', exportBuilt.outputFiles[0].text)(exportModule, exportModule.exports);
const { exportHistoryRecord, exportHistoryToApp, completeHistoryRecord } = exportModule.exports;
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
  assert.equal(prepared.strings.every(range => Object.keys(range).sort().join(',') === 'end,marker,prefix,start'), true);
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
test('giant projected templates stream original strings, escaped pretty input and cross-block matches', async () => {
  const content = 'head中😀' + 'x'.repeat(170000) + 'NEEDLE' + '\n'.repeat(2000) + 'tail';
  const raw = JSON.stringify({ hidden: 'NEEDLE'.repeat(12000), body: content, second: 'NEXT' });
  const bytes = new TextEncoder().encode(raw), start = 123;
  const ref = { start, end: start + bytes.length, cursor: 'original', item: null, missing_start: false, missing_end: false };
  const reads = [];
  const io = {
    record: async (cursor, from) => {
      assert.equal(cursor, 'joined'); reads.push(from);
      const data = bytes.subarray(from - start, Math.min(from - start + 65536, bytes.length));
      return { offset: from, data: Buffer.from(data).toString('base64'), next: from + data.length < ref.end ? from + data.length : null };
    },
    project: async (_cursor, first, end, scaffold) => {
      assert.deepEqual([first, end], [ref.start, ref.end]);
      const value = scaffold ? JSON.parse(scaffold) : null;
      return { record: { ...ref, cursor: 'joined' }, transforms: [], complete: true,
        entries: value ? [text('projection:0', value.body + '\n' + value.second)] : [] };
    },
  };
  const scan = findRecord(query('needle'), ref, 'before', io);
  const first = await scan.next();
  assert.equal(first.done, false); assert.equal(first.value.from, content.indexOf('NEEDLE'));
  assert.equal(first.value.record.cursor, 'joined'); assert.equal(first.value.snippet.includes('NEEDLE'), true);
  assert.deepEqual(await scan.next(), { done: true, value: true });
  assert.equal(reads.length >= Math.ceil(bytes.length / 65536) + 1, true, 'normalization and body reads both use bounded original chunks');
  const crossing = findRecord(query('tail\nnext'), ref, 'before', io);
  assert.equal((await crossing.next()).value.from, content.length - 4);
  await crossing.return();
  const prettyIO = { ...io, project: async (_cursor, _first, _end, scaffold) => ({ record: { ...ref, cursor: 'joined' }, transforms: [], complete: true,
    entries: scaffold ? [note('projection:0', { sessionUpdate: 'tool_call', toolCallId: 'c', title: 'Call', rawInput: { text: JSON.parse(scaffold).body } })] : [] }) };
  const escaped = findRecord(query(String.raw`NEEDLE\n\n`), ref, 'before', prettyIO);
  assert.equal((await escaped.next()).done, false, 'pretty raw input searches its actual escaped displayed spelling');
  await escaped.return();
  const absent = findRecord(query('hidden'), ref, 'before', io);
  assert.deepEqual(await absent.next(), { done: true, value: true }, 'JSON keys never become searchable body text');
});
test('normalizer facts decode one encoded tool input and preserve invalid-input fallback', async () => {
  const command = 'echo ' + '中😀'.repeat(22000) + ' COMMAND_NEEDLE';
  const argumentsText = JSON.stringify({ hidden: 'HIDDEN_NEEDLE'.repeat(10000), command });
  const make = (argumentsText, failInner = false) => {
    const bytes = new TextEncoder().encode(JSON.stringify({ payload: { arguments: argumentsText } }));
    const ref = { start: 0, end: bytes.length, cursor: 'encoded-tool-input', item: null, missing_start: false, missing_end: false };
    let projections = 0;
    return { ref, io: {
      record: async (_cursor, from) => {
        if (failInner && projections === 1) throw new Error('source changed during encoded preparation');
        const data = bytes.subarray(from, from + 65536);
        return { offset: from, data: Buffer.from(data).toString('base64'), next: from + data.length < bytes.length ? from + data.length : null };
      },
      project: async (_cursor, _start, _end, scaffold) => {
        if (!scaffold) return { record: ref, entries: [], transforms: [], complete: false };
        projections++;
        let input = JSON.parse(scaffold).payload.arguments, parsed = false;
        try { input = JSON.parse(input); parsed = true; } catch { /* actual normalizer's string fallback */ }
        return { record: ref, complete: true, transforms: [{ kind: 'json_string', pointer: '/payload/arguments', parsed }],
          entries: [note('projection:0', { sessionUpdate: 'tool_call', toolCallId: 'c', title: 'Tool', rawInput: input })] };
      },
    }, projections: () => projections };
  };
  const found = make(argumentsText);
  const scan = findRecord(query('command_needle'), found.ref, 'before', found.io);
  const hit = await scan.next();
  assert.equal(hit.done, false); assert.equal(hit.value.from, command.indexOf('COMMAND_NEEDLE'));
  assert.deepEqual(await scan.next(), { done: true, value: true });
  assert.equal(found.projections(), 2, 'selected string is prepared then passed back through the same normalizer');
  const hidden = make(argumentsText);
  assert.deepEqual(await findRecord(query('HIDDEN_NEEDLE'), hidden.ref, 'before', hidden.io).next(), { done: true, value: true },
    'a command-only displayed input must not search another encoded JSON field');
  const fallback = make('not JSON ' + 'x'.repeat(150000) + ' FALLBACK_NEEDLE');
  const raw = findRecord(query('fallback_needle'), fallback.ref, 'before', fallback.io);
  assert.equal((await raw.next()).done, false);
  assert.deepEqual(await raw.next(), { done: true, value: true });
  assert.equal(fallback.projections(), 1, 'invalid inner syntax retains the exact original string fallback');
  const changed = make(argumentsText, true);
  await assert.rejects(findRecord(query('needle'), changed.ref, 'before', changed.io).next(), /source changed during encoded/);
});
test('normalizer output policy trims Unicode whitespace and searches only its visible UTF-8 prefix', async () => {
  const stream = async function* (text) { yield* text; };
  assert.equal(await limitedToolOutput(stream('1234567\u3000 \tZ'), 8, ' CUT'), '1234567 CUT', 'a later short whitespace must not jump over the first UTF-8 cut boundary');
  assert.equal(await limitedToolOutput(stream('\uFEFFhead \u0085'), 16, ' CUT'), '\uFEFFhead', 'Rust trim keeps BOM and removes NEL');
  const output = ' \u0085' + '中😀'.repeat(12000) + ' HIDDEN_TAIL_NEEDLE  ';
  const bytes = new TextEncoder().encode(JSON.stringify({ content: output }));
  assert.equal(bytes.length > 65536, true, 'the output actually exceeds the inline record boundary');
  const ref = { start: 0, end: bytes.length, cursor: 'bounded-tool-output', item: null, missing_start: false, missing_end: false };
  const io = {
    record: async (_cursor, from) => {
      const data = bytes.subarray(from, from + 65536);
      return { offset: from, data: Buffer.from(data).toString('base64'), next: from + data.length < bytes.length ? from + data.length : null };
    },
    project: async (_cursor, _start, _end, scaffold) => ({ record: ref, complete: true,
      transforms: [{ kind: 'tool_output', item: 7, max_bytes: 16384, suffix: '\n… visible cut' }],
      entries: scaffold ? [note('projection:7', { sessionUpdate: 'tool_call_update', toolCallId: 'call',
        content: [{ type: 'content', content: { type: 'text', text: JSON.parse(scaffold).content } }] })] : [] }),
  };
  const head = findRecord(query('中😀'), ref, 'before', io);
  assert.equal((await head.next()).value.from, 0, 'Rust White_Space trim includes NEL'); await head.return();
  assert.deepEqual(await findRecord(query('HIDDEN_TAIL_NEEDLE'), ref, 'before', io).next(), { done: true, value: true });
  assert.equal((await findRecord(query('visible cut'), ref, 'before', io).next()).done, false, 'the existing marked cut suffix is visible text');
});
test('context and full-part copy share the normalized source stream, with an explicit copy limit', async () => {
  const body = 'start ' + '中😀'.repeat(25000) + ' NEEDLE tail';
  const bytes = new TextEncoder().encode(JSON.stringify({ body }));
  const ref = { start: 0, end: bytes.length, cursor: 'copy-source', item: null, missing_start: false, missing_end: false };
  const io = {
    page: async () => { throw new Error('giant copy must use its exact original reference'); },
    record: async (_cursor, from) => {
      const data = bytes.subarray(from, from + 65536);
      return { offset: from, data: Buffer.from(data).toString('base64'), next: from + data.length < bytes.length ? from + data.length : null };
    },
    project: async (_cursor, _start, _end, scaffold) => ({ record: ref, transforms: [], complete: true,
      entries: scaffold ? [text('projection:0', JSON.parse(scaffold).body)] : [] }),
  };
  const at = body.indexOf('NEEDLE');
  const hit = { before: 'before', entry: '0:0', part: 0, from: at, to: at + 6, snippet: '', record: ref };
  const context = await historyFindAction('context', hit, io);
  assert.equal(context.complete, false); assert.equal(context.text.includes('NEEDLE'), true);
  assert.equal(context.text.slice(hit.from - context.from, hit.to - context.from), 'NEEDLE');
  assert.equal(new TextEncoder().encode(context.text).length < 2048, true);
  await assert.rejects(historyFindAction('copy', hit, io), error => error.message === 'too_large');
});
test('original-record export preserves bytes across UTF-8 boundaries and aborts partial targets', async () => {
  const bytes = new TextEncoder().encode('中😀'.repeat(30000));
  const reference = { cursor: 'export-source', start: 100, end: bytes.length + 100 };
  const writes = [];
  let closed = 0, aborted = 0, reads = 0;
  const source = { record: async (cursor, from) => {
    reads++; assert.equal(cursor, 'export-source');
    const data = bytes.subarray(from - 100, from - 100 + 65536);
    return { offset: from, data: Buffer.from(data).toString('base64'), next: from + data.length < reference.end ? from + data.length : null };
  } };
  const target = { write: async data => writes.push(Buffer.from(data)), close: async () => { closed++; }, abort: async () => { aborted++; } };
  await exportHistoryRecord(source, reference, target, new AbortController().signal);
  assert.equal(Buffer.concat(writes).equals(Buffer.from(bytes)), true); assert.equal(closed, 1); assert.equal(aborted, 0);
  assert.equal(writes.every(chunk => chunk.length <= 65536), true);
  const stop = new AbortController(); reads = 0; writes.length = 0;
  await assert.rejects(exportHistoryRecord(source, reference, { ...target, write: async data => { writes.push(Buffer.from(data)); stop.abort(); } }, stop.signal), /cancelled/);
  assert.equal(reads, 1); assert.equal(closed, 1); assert.equal(aborted, 1);
  await assert.rejects(exportHistoryRecord({ record: async () => ({ offset: 100, data: 'eA==', next: null }) }, reference, target, new AbortController().signal), /continuation/);
  assert.equal(aborted, 2);
});
test('raw export finds both physical boundaries without parsing or retaining all fragments', async () => {
  const ref = (start, end, missing_start, missing_end, cursor) => ({ start, end, missing_start, missing_end, cursor, item: null });
  const basis = ref(100, 200, true, true, 'basis');
  const calls = [];
  const io = { page: async (cursor, newer) => {
    calls.push([cursor, newer]);
    const piece = !newer ? cursor === 'basis' ? ref(50, 100, true, true, 'left') : ref(0, 50, false, true, 'start')
      : cursor === 'basis' ? ref(100, 200, true, true, 'basis') : ref(200, 300, true, false, 'end');
    return {...page([], [piece]), newer: cursor === 'basis' ? 'next' : 'end', has_more: cursor === 'basis'};
  }, project: async (cursor, start, end, scaffold) => {
    assert.deepEqual([cursor, start, end, scaffold], ['basis', 0, 300, null]);
    return { record: ref(0, 300, true, true, 'joined') };
  } };
  const result = await completeHistoryRecord(io, basis, new AbortController().signal);
  assert.deepEqual(calls, [['basis', false], ['left', false], ['basis', true], ['next', true]]);
  assert.deepEqual([result.start, result.end, result.cursor, result.missing_start, result.missing_end], [0, 300, 'joined', false, false]);
  const malformedButWhole = ref(0, 11, false, false, 'invalid-json');
  assert.equal(await completeHistoryRecord({page: () => { throw new Error('do not parse a known original'); }, project: () => { throw new Error('no body projection'); }}, malformedButWhole, new AbortController().signal), malformedButWhole);
});
test('raw export rejects reset, unfinished records and cancellation instead of guessing a complete boundary', async () => {
  const basis = {start: 100, end: 200, cursor: 'basis', missing_start: true, missing_end: true, item: null};
  const unused = () => { throw new Error('invalid source must not be projected'); };
  await assert.rejects(completeHistoryRecord({page: async () => ({...page([]), reset: true}), project: unused}, basis, new AbortController().signal), /source changed/);
  await assert.rejects(completeHistoryRecord({page: async () => ({...page([], [{...basis, start: 200, end: 300, missing_start: false}]), has_more: false}), project: unused}, {...basis, missing_start: false}, new AbortController().signal), /incomplete/);
  const waiting = deferred(), entered = deferred(), stop = new AbortController(); let reads = 0;
  const work = completeHistoryRecord({page: async () => { reads++; entered.resolve(); return waiting.promise; }, project: unused}, basis, stop.signal);
  const rejected = assert.rejects(work, /cancelled/);
  await entered.promise; stop.abort(); waiting.resolve(page([])); await rejected;
  assert.equal(reads, 1);
});
test('valid JSON retains only source-proven physical record boundaries', async () => {
  const bytes = new TextEncoder().encode(JSON.stringify({body: 'needle' + 'x'.repeat(70000)}) + '\n');
  const basis = {start: 0, end: bytes.length, cursor: 'basis', missing_start: false, missing_end: false, item: null};
  const io = {record: async (_cursor, from) => {const data = bytes.subarray(from, from + 65536);return {offset: from, data: Buffer.from(data).toString('base64'), next: from + data.length < bytes.length ? from + data.length : null};},
    project: async (_cursor, _start, _end, scaffold) => ({record: {...basis, cursor: 'joined', missing_start: true, missing_end: true}, transforms: [], complete: true, entries: scaffold ? [text('projection:0', JSON.parse(scaffold).body)] : []}),
  };
  for (const known of [true, false]) {
    const scanner = findRecord(query('needle'), {...basis, missing_start: !known, missing_end: !known}, 'before', io);
    const result = await scanner.next();
    assert.equal(result.done, false);
    assert.deepEqual([result.value.record.missing_start, result.value.record.missing_end], [!known, !known]);
    await scanner.return();
  }
});
test('native export owns late start ACKs and cancellation through the OS presenter', async () => {
  const ready = { moved: 12, total: 12, ready: true, cancelled: false, error: null };
  const ref = { start: 100, end: 112 };
  const start = deferred(), cleanup = deferred();
  const calls = [];
  const stop = new AbortController();
  let settled = false;
  const pending = exportHistoryToApp({
    start: () => { calls.push('start'); return start.promise; },
    state: async () => { throw new Error('a retired job must not poll'); },
    cancel: () => { calls.push('cancel'); return cleanup.promise; },
    save: async () => { calls.push('save'); return true; },
  }, ref, stop.signal).finally(() => { settled = true; });
  const rejected = assert.rejects(pending, /cancelled/);
  stop.abort(); start.resolve(ready);
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(calls, ['start', 'cancel']);
  assert.equal(settled, false, 'leaving waits for real cleanup of its late job');
  cleanup.resolve(); await rejected;
  const presentation = deferred(), reached = deferred();
  const duringSave = new AbortController(); let cancels = 0;
  const showing = exportHistoryToApp({ start: async () => ready,
    state: async () => { throw new Error('completed job must not poll'); },
    cancel: async () => { cancels++; },
    save: () => { reached.resolve(); return presentation.promise; },
  }, ref, duringSave.signal);
  const showingRejected = assert.rejects(showing, /cancelled/);
  await reached.promise; duringSave.abort(); presentation.resolve(true); await showingRejected;
  assert.equal(cancels, 1, 'abort and finally share one cancellation');
  assert.equal(await exportHistoryToApp({ start: async () => ready, state: async () => ready,
    cancel: async () => { throw new Error('successful presentation must not cancel'); }, save: async () => false,
  }, ref, new AbortController().signal), false, 'dismissed Save panel is not reported as a completed export');
});
test('repeated cached raw ranges still yield the worker task queue within one scan budget', async () => {
  const bytes = new TextEncoder().encode(JSON.stringify({ body: 'x'.repeat(12000) }));
  const ref = { start: 0, end: bytes.length, cursor: 'cached', item: null, missing_start: false, missing_end: false };
  let taskRan = false, reads = 0;
  const timer = setTimeout(() => { taskRan = true; }, 0);
  const io = {
    record: async () => { reads++; return { offset: 0, data: Buffer.from(bytes).toString('base64'), next: null }; },
    project: async (_cursor, _start, _end, scaffold) => ({ record: ref, transforms: [], complete: true,
      entries: scaffold ? [text('projection:0', Array.from({ length: 64 }, () => JSON.parse(scaffold).body).join('\n'))] : [] }),
  };
  try {
    assert.deepEqual(await findRecord(query('absent'), ref, 'before', io).next(), { done: true, value: true });
    assert.equal(reads, 1, 'all repeated body visits deliberately reuse the same original chunk');
    assert.equal(taskRan, true, 'cached-range scans must service cancellation tasks before finishing');
  } finally { clearTimeout(timer); }
});
test('giant searches expose source errors and incomplete structure, with no full text fallback', async () => {
  const ref = { start: 0, end: 2, cursor: 'raw', item: null, missing_start: false, missing_end: false };
  const io = { project: async (_c, _s, _e, scaffold) => ({ record: ref, transforms: [], complete: false, entries: [] }),
    record: async () => ({ offset: 0, data: Buffer.from('{]').toString('base64'), next: null }) };
  assert.deepEqual(await findRecord(query('x'), ref, 'before', io).next(), { done: true, value: false });
  await assert.rejects(findRecord(query('x'), ref, 'before', { ...io, record: async () => { throw new Error('source changed'); } }).next(), /source changed/);
  await assert.rejects(findRecord(query('x'), ref, 'before', { ...io, record: async () => ({ offset: 1, data: 'e30=', next: null }) }).next(), /range or byte/);
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
test('actual worker joins fragmented source ranges and resumes giant hits through owned RPC leases', async () => {
  const body = '中😀' + ('needle ' + 'x'.repeat(3000)).repeat(70);
  const bytes = new TextEncoder().encode(JSON.stringify({ hidden: 'needle'.repeat(5000), body }));
  const middle = Math.floor(bytes.length / 2);
  const ref = { start: 0, end: bytes.length, item: null, cursor: 'joined', missing_start: false, missing_end: false };
  const calls = [], cancelled = [];
  let leases = 0, arrived = deferred(), last;
  const source = historySource('giant-binding', {
    begin: async () => `giant${++leases}`, cancel: async key => { cancelled.push(key); },
    page: async (_id, _key, before) => before === null
      ? { ...page([], [{ ...ref, start: middle, cursor: 'right-fragment', missing_start: true }]), older: 'left-page' }
      : { ...page([], [{ ...ref, end: middle, cursor: 'left-fragment', missing_end: true }]), older: null },
    project: async (_id, key, cursor, start, end, scaffold) => {
      calls.push(['project', key, cursor, start, end]);
      assert.deepEqual([start, end], [0, bytes.length]);
      return { record: ref, transforms: [], complete: true, entries: scaffold ? [text('projection:7', JSON.parse(scaffold).body)] : [] };
    },
    record: async (_id, _key, cursor, from) => {
      assert.equal(cursor, 'joined'); calls.push(['record', from]);
      const data = bytes.subarray(from, from + 65536);
      return { offset: from, data: Buffer.from(data).toString('base64'), next: from + data.length < bytes.length ? from + data.length : null };
    },
  });
  const reader = historyFind(source, query('needle'), state => {
    last = state; if (!state.busy && (state.hits.length || state.done || state.error)) arrived.resolve(state);
  });
  try {
    reader.start();
    const first = await arrived.promise;
    assert.equal(first.error, null); assert.equal(first.hits.length, 32);
    assert.equal(first.scannedPages, 2); assert.equal(first.hits[0].entry, '0:7');
    arrived = deferred(); reader.next(); const second = await arrived.promise;
    arrived = deferred(); reader.next(); const third = await arrived.promise;
    assert.deepEqual([first.hits.length, second.hits.length, third.hits.length], [32, 32, 6]);
    assert.deepEqual([...first.hits, ...second.hits, ...third.hits].map(hit => hit.from), Array.from({ length: 70 }, (_, i) => 3 + i * 3007));
    assert.equal(third.done, true); assert.equal(third.complete, true);
    assert.equal(calls.filter(call => call[0] === 'project').length, 2);
    for (const call of calls.filter(call => call[0] === 'project')) assert.equal(cancelled.filter(key => key === call[1]).length, 1);
    const cleanup = await reader.dispose(); assert.equal(cleanup.failures, 0);
    const count = calls.length; await reader.next(); assert.equal(calls.length, count);
  } finally { await reader.dispose(); }
});
test('a changed raw source clears already displayed giant search hits', async () => {
  const bytes = new TextEncoder().encode(JSON.stringify({ body: 'needle '.repeat(32) + 'x'.repeat(150000) + 'needle' }));
  const ref = { start: 0, end: bytes.length, cursor: 'changed-record', item: null, missing_start: false, missing_end: false };
  let changed = false, arrived = deferred();
  const source = historySource('changing-raw', {
    begin: async () => crypto.randomUUID(), cancel: async () => {},
    page: async () => ({ ...page([], [ref]), older: null }),
    project: async (_id, _key, _cursor, _start, _end, scaffold) => ({ record: ref, transforms: [], complete: true,
      entries: scaffold ? [text('projection:0', JSON.parse(scaffold).body)] : [] }),
    record: async (_id, _key, _cursor, from) => {
      if (changed) throw new Error('source changed while reading');
      const data = bytes.subarray(from, from + 65536);
      return { offset: from, data: Buffer.from(data).toString('base64'), next: from + data.length < bytes.length ? from + data.length : null };
    },
  });
  const reader = historyFind(source, query('needle'), state => { if (state.hits.length || state.error) arrived.resolve(state); });
  try {
    reader.start(); assert.equal((await arrived.promise).hits.length, 32);
    changed = true; arrived = deferred(); reader.next();
    const failed = await arrived.promise;
    assert.match(failed.error, /source changed/); assert.deepEqual(failed.hits, []); assert.equal(failed.complete, false);
  } finally { await reader.dispose(); }
});
test('actual giant worker disposal cancels a held raw read without posting late matches', async () => {
  const entered = deferred(), held = deferred(), cancelled = [], states = [];
  const ref = { start: 0, end: 200000, item: null, cursor: 'raw', missing_start: false, missing_end: false };
  let lease = 0;
  const source = historySource('giant-cancel', {
    begin: async () => `cancel${++lease}`, cancel: async key => { cancelled.push(key); },
    page: async () => ({ ...page([], [ref]), older: null }),
    project: async () => ({ record: ref, entries: [], transforms: [], complete: false }),
    record: async (_id, key) => { entered.resolve(key); return held.promise; },
  });
  const reader = historyFind(source, query('needle'), state => states.push(state));
  try {
    reader.start(); const key = await entered.promise;
    const closing = reader.dispose(); const count = states.length;
    held.resolve({ offset: 0, data: Buffer.from('"needle"').toString('base64'), next: null });
    await closing;
    assert.equal(cancelled.filter(value => value === key).length, 1);
    assert.equal(states.length, count);
  } finally { held.resolve(null); await reader.dispose(); }
});
test('actual worker context and copy re-read the source independently of the rendered cache', async () => {
  let changed = false, arrived = deferred(), last;
  const reads = [];
  const body = 'source context NEEDLE 原文';
  const source = historySource('action-binding', {
    begin: async () => crypto.randomUUID(), cancel: async () => {},
    page: async (_id, _key, cursor) => {
      reads.push(cursor);
      assert.equal(cursor === null || cursor === 'exclusive-source-fence', true);
      return { ...page([text('17:0', body)]), older: null, reset: changed };
    },
    record: async () => { throw new Error('normal source part does not require raw expansion'); },
  });
  const reader = historyFind(source, query('needle'), state => { last = state; if (state.hits.length || state.error) arrived.resolve(state); });
  try {
    reader.start(); const first = await arrived.promise, hit = first.hits[0];
    assert.equal((await reader.inspect(hit, 'context')).text, body);
    const copy = await reader.inspect(hit, 'copy');
    assert.equal(copy.text, body); assert.equal(copy.complete, true);
    assert.deepEqual(reads, [null, 'exclusive-source-fence', 'exclusive-source-fence']);
    changed = true;
    await assert.rejects(reader.inspect(hit, 'context'), /source_changed/);
    assert.deepEqual(last.hits, []);
    await assert.rejects(reader.inspect(hit, 'copy'), /no longer active/);
  } finally { await reader.dispose(); }
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
const selectedChecks = only ? checks.filter(([name]) => name === only) : checks;
assert.equal(selectedChecks.length > 0, true, 'the selected history search fixture must exist');
try {
  await Promise.race([
    (async () => { for (const [name, run] of selectedChecks) { await run(); console.log('PASS', name); } })(),
    new Promise((_, reject) => { const timer = setTimeout(() => reject(new Error('history find fixture exceeded 10s')), 10_000); timer.unref(); }),
  ]);
} finally {
  for (const worker of ownedWorkers) await worker.terminate();
  await Promise.all(workerExits);
  globalThis.Worker = OriginalWorker;
}
assert.equal(ownedWorkers.size, 0);
console.log(`${selectedChecks.length} source-backed history find checks passed; actual worker threads, no browser/native-normalization claim`);
