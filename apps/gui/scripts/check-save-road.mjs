// Exercise the production save coordinator with content-derived digests and
// controlled transport completion. No DOM, server or user files are used.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { transformSync } from 'esbuild';

const source = readFileSync(new URL('../src/lib/save-road.ts', import.meta.url), 'utf8');
const compiled = transformSync(source, { loader: 'ts', format: 'cjs', target: 'node18' }).code;
const digest = text => createHash('sha256').update(text).digest('hex');
const settle = () => new Promise(resolve => setImmediate(resolve));

function fixture(initial = 'A') {
  let disk = initial, now = 0, timerId = 0, delayReads = false, readableDigest = true;
  const requests = [], reads = [], timers = new Map(), landed = [];
  const snapshot = () => ({ text: disk, digest: readableDigest ? digest(disk) : '', at_ms: now });
  const api = {
    said: error => String(error),
    fetchFileText: () => delayReads ? new Promise(resolve => reads.push(resolve)) : Promise.resolve(snapshot()),
    saveFileText: (machine, path, was, text) => new Promise((resolve, reject) => {
      requests.push({ machine, path, was, text, resolve, reject });
    }),
  };
  const exports = {};
  const context = {
    module: { exports }, exports,
    require(name) {
      if (name === 'react') return { useSyncExternalStore() { throw Error('No React hooks in this check'); } };
      if (name === '@/api') return api;
      throw Error(`Unexpected dependency: ${name}`);
    },
    Date: class extends Date { static now() { return now; } },
    setTimeout(fn, delay) { const id = ++timerId; timers.set(id, { at: now + delay, fn }); return id; },
    clearTimeout(id) { timers.delete(id); },
  };
  vm.runInNewContext(compiled, context, { filename: 'save-road.ts' });
  const road = context.module.exports;
  road.onAnyLanded(value => landed.push(value));
  const write = (text, by = 'first', was = digest(disk), how = {}) =>
    road.write('machine', '/file', { text, by, was }, how).then(value => ({ value }), error => ({ error }));
  const reply = async (index, failure) => {
    const request = requests[index];
    assert(request, `request ${index} was submitted`);
    if (failure) request.reject(new Error(failure));
    else if (request.was !== digest(disk)) request.reject(new Error('compare-and-write conflict'));
    else { disk = request.text; request.resolve({ digest: digest(disk), at_ms: now }); }
    await settle();
  };
  const save = async (text, by = 'first', was = digest(disk)) => {
    const index = requests.length, pending = write(text, by, was);
    await reply(index);
    const result = await pending;
    assert(!result.error, `save ${text}: ${JSON.stringify(result.error)}`);
    return result.value;
  };
  const advance = async (ms) => {
    const end = now + ms;
    for (;;) {
      const next = [...timers].filter(([, t]) => t.at <= end).sort((a, b) => a[1].at - b[1].at)[0];
      if (!next) break;
      now = next[1].at; timers.delete(next[0]); next[1].fn(); await settle();
    }
    now = end;
  };
  return {
    road, requests, timers, landed, write, reply, save, advance,
    get disk() { return disk; }, set disk(value) { disk = value; },
    state: () => road.roadOf('machine', '/file'),
    discard: () => road.discard('machine', '/file'),
    reopen: () => road.reopened('machine', '/file'),
    holdReads: () => { delayReads = true; },
    withoutDigest: () => { readableDigest = false; },
    replyRead: async index => { assert(reads[index], `read ${index} was submitted`); reads[index](snapshot()); await settle(); },
  };
}

const cases = [];
const check = (name, run) => cases.push([name, run]);
check('content cycle A → B → C → A → D keeps saving', async () => {
  const f = fixture();
  for (const text of ['B', 'C', 'A', 'D']) await f.save(text);
  assert.equal(f.disk, 'D'); assert.equal(f.state().trouble, null);
});
check('queued and remounted buffers follow their own confirmed head', async () => {
  const f = fixture(), base = digest('A');
  const first = f.write('B', 'first', base), next = f.write('C', 'first', base);
  await f.reply(0); await first;
  await f.reply(1); assert(!(await next).error);
  await f.save('A', 'first', base);
  await f.save('D', 'first', base);
  assert.equal(f.disk, 'D');
});
check('independent editors continue from their own read and conflict on stale text', async () => {
  const f = fixture();
  await f.save('B', 'one'); await f.save('C', 'two');
  const stale = f.write('D', 'one', digest('B'));
  await f.reply(2); assert.equal((await stale).error.kind, 'clash');
  assert.equal(f.disk, 'C'); await f.save('E', 'two');
  assert.equal(f.disk, 'E');
});
check('a real external edit is not advanced through another digest', async () => {
  const f = fixture(); await f.save('B'); f.disk = 'external';
  const pending = f.write('C', 'first', digest('A'));
  await f.reply(1); assert.equal((await pending).error.kind, 'clash');
  assert.equal(f.disk, 'external'); assert.equal(f.timers.size, 0);
});
check('a fresh overwrite read starts its own line instead of forwarding historical content', async () => {
  const f = fixture(); await f.save('B'); f.disk = 'A';
  const read = await f.road.readForOverwrite('machine', '/file');
  const pin = read.pin('C'); assert(pin);
  assert.equal(pin.was, digest('A')); assert.notEqual(pin.by, 'first');
  const pending = f.write(pin.text, pin.by, pin.was, {asked: true});
  await f.reply(1); assert(!(await pending).error); assert.equal(f.disk, 'C');
  await f.save('D', pin.by, pin.was); assert.equal(f.disk, 'D');
});
check('an ordinary asked save does not turn a stale pin into a fresh overwrite', async () => {
  const f = fixture(); await f.save('B'); f.disk = 'A';
  const pending = f.write('C', 'first', digest('A'), {asked: true});
  await f.reply(1); assert.equal((await pending).error.kind, 'clash');
  assert.equal(f.disk, 'A'); assert.equal(f.requests[1].was, digest('B'));
});
for (const stage of ['pending', 'resolved']) {
  check(`discard revokes a ${stage} overwrite read even after reopening`, async () => {
    const f = fixture();
    if (stage === 'pending') f.holdReads();
    const pending = f.road.readForOverwrite('machine', '/file');
    let read;
    if (stage === 'resolved') read = await pending;
    f.discard(); f.reopen();
    if (stage === 'pending') { await f.replyRead(0); read = await pending; }
    assert.equal(read.pin('C'), null); assert.equal(f.requests.length, 0); assert.equal(f.disk, 'A');
  });
}
check('a fresh overwrite still refuses a file changed after its read', async () => {
  const f = fixture(); await f.save('B');
  const read = await f.road.readForOverwrite('machine', '/file');
  f.disk = 'external';
  const pin = read.pin('C'); assert(pin);
  const pending = f.write(pin.text, pin.by, pin.was, {asked: true});
  await f.reply(1); assert.equal((await pending).error.kind, 'clash');
  assert.equal(f.disk, 'external');
});
check('an incomplete overwrite read cannot produce an editable pin', async () => {
  const f = fixture(); f.withoutDigest();
  const read = await f.road.readForOverwrite('machine', '/file');
  assert.equal(read.pin('C'), null); assert.equal(f.requests.length, 0);
});
check('coalescing never acknowledges a different editor with somebody else’s text', async () => {
  const f = fixture(), base = digest('A');
  const first = f.write('B', 'one', base);
  const other = f.write('C', 'two', base);
  const later = f.write('D', 'one', base);
  await f.reply(0); assert(!(await first).error);
  assert.equal(f.requests[1].text, 'C'); await f.reply(1);
  assert.equal((await other).error.kind, 'clash');
  assert.equal(f.requests[2].text, 'D'); await f.reply(2);
  assert(!(await later).error); assert.equal(f.disk, 'D');
});
check('same editor coalesces pending text and answers all waiting saves', async () => {
  const f = fixture(), base = digest('A');
  const first = f.write('B', 'one', base), second = f.write('C', 'one', base), last = f.write('D', 'one', base);
  await f.reply(0); await first; assert.equal(f.requests[1].text, 'D');
  await f.reply(1);
  assert.equal((await second).value.text, 'D'); assert.equal((await last).value.text, 'D');
  assert.equal(f.requests.length, 2);
});
check('ordinary network failures retry with backoff and settle on success', async () => {
  const f = fixture(), pending = f.write('B'); await f.reply(0, 'offline');
  assert.equal((await pending).error.kind, 'failed');
  await f.advance(1999); assert.equal(f.requests.length, 1);
  await f.advance(1); assert.equal(f.requests.length, 2); await f.reply(1, 'still offline');
  await f.advance(3999); assert.equal(f.requests.length, 2);
  await f.advance(1); await f.reply(2); assert.equal(f.disk, 'B'); assert.equal(f.state().trouble, null);
});
check('a lost successful response is reconciled by content', async () => {
  const f = fixture(), pending = f.write('B'); f.disk = 'B';
  await f.reply(0, 'response lost'); assert.equal((await pending).value.text, 'B');
  assert.equal(f.timers.size, 0); await f.save('C');
});
for (const action of ['discard', 'forget', 'discardAll']) {
  check(`${action} while a write flies cannot restart failed work`, async () => {
    const f = fixture(), pending = f.write('B');
    if (action === 'discardAll') f.road.discardAll(); else f.road[action]('machine', '/file');
    await f.reply(0, 'offline'); await pending;
    assert.equal(f.timers.size, 0); assert.equal(f.state().trouble, null);
    await f.advance(120_000); assert.equal(f.requests.length, 1); assert.equal(f.disk, 'A');
  });
}
check('discard cancels an already scheduled retry', async () => {
  const f = fixture(), pending = f.write('B'); await f.reply(0, 'offline'); await pending;
  f.discard(); await f.advance(120_000); assert.equal(f.requests.length, 1);
});
check('a discarded successful flight reports disk truth without reviving UI callbacks', async () => {
  const f = fixture(); let orphan = 0;
  const pending = f.write('B', 'first', digest('A'), { asked: true, orphan: () => orphan++ });
  f.discard(); await f.reply(0); assert(!(await pending).error);
  assert.equal(f.disk, 'B'); assert.equal(f.landed.length, 1);
  assert.equal(orphan, 0); assert.equal(f.state().confirmed, false); assert.equal(f.state().trouble, null);
});
check('reopening after discard keeps new queued work when the old flight fails', async () => {
  const f = fixture(), old = f.write('B'); f.discard(); f.reopen();
  const next = f.write('new edit', 'new-line');
  await f.reply(0, 'offline'); await old;
  assert.equal(f.state().trouble, null); assert.equal(f.requests[1].text, 'new edit');
  await f.reply(1); assert(!(await next).error); assert.equal(f.disk, 'new edit');
  assert.equal(f.timers.size, 0);
});
check('a fresh editor conflicts if the discarded flight did reach disk', async () => {
  const f = fixture(), old = f.write('B'); f.discard(); f.reopen();
  const next = f.write('new edit', 'new-line'); await f.reply(0); await old;
  await f.reply(1); assert.equal((await next).error.kind, 'clash'); assert.equal(f.disk, 'B');
  await f.save('new edit', 'new-line'); assert.equal(f.disk, 'new edit');
});
check('reopened identical text is not merged into a cancelled flight', async () => {
  const f = fixture(), old = f.write('B'); f.discard(); f.reopen();
  const next = f.write('B'); await f.reply(0, 'offline'); await old;
  assert.equal(f.requests.length, 2); await f.reply(1); assert(!(await next).error);
  assert.equal(f.timers.size, 0); assert.equal(f.disk, 'B');
});
check('forget resets old lineage before a merged buffer is submitted', async () => {
  const f = fixture(); await f.save('B'); f.disk = 'A';
  f.road.forget('machine', '/file'); await f.save('merged', 'first', digest('A'));
  assert.equal(f.disk, 'merged');
});
for (const reachedDisk of [false, true]) {
  check(`discard during error reconciliation stays discarded (disk write ${reachedDisk})`, async () => {
    const f = fixture(); let orphan = 0;
    f.holdReads();
    const pending = f.write('B', 'first', digest('A'), { asked: true, orphan: () => orphan++ });
    await f.reply(0, 'response lost'); f.discard();
    if (reachedDisk) f.disk = 'B';
    await f.replyRead(0);
    const result = await pending;
    assert.equal(Boolean(result.value), reachedDisk); assert.equal(f.landed.length, reachedDisk ? 1 : 0);
    assert.equal(f.timers.size, 0); assert.equal(f.state().trouble, null);
    assert.equal(f.state().confirmed, false); assert.equal(orphan, 0);
  });
}
check('discarded leaving autosave is refused, but an explicit save can proceed', async () => {
  const f = fixture(); f.discard();
  assert((await f.write('discarded')).error); assert.equal(f.requests.length, 0);
  const explicit = f.write('wanted', 'fresh', digest('A'), { asked: true });
  await f.reply(0); assert(!(await explicit).error); assert.equal(f.disk, 'wanted');
  assert.equal(f.state().confirmed, true);
});
check('new work after discard retains its own failure and retry', async () => {
  const f = fixture(), old = f.write('B'); f.discard(); f.reopen();
  const next = f.write('new edit', 'new-line'); await f.reply(0, 'old offline'); await old;
  await f.reply(1, 'new offline'); assert.equal((await next).error.kind, 'failed');
  assert.match(f.state().trouble.said, /new offline/);
  await f.advance(2000); assert.equal(f.requests[2].text, 'new edit');
  await f.reply(2); assert.equal(f.disk, 'new edit'); assert.equal(f.state().trouble, null);
});
check('discard settles every queued editor without dispatching their writes', async () => {
  const f = fixture(), first = f.write('B'), second = f.write('C', 'two'), third = f.write('D', 'three');
  f.discard(); assert((await second).error); assert((await third).error);
  await f.reply(0, 'offline'); await first;
  assert.equal(f.requests.length, 1); assert.equal(f.state().busy, false); assert.equal(f.timers.size, 0);
});
check('flushAll waits for the last independent queued editor', async () => {
  const f = fixture(), base = digest('A');
  const first = f.write('B', 'one', base), second = f.write('C', 'two', base), third = f.write('D', 'one', base);
  let flushed = false; const flushing = f.road.flushAll().then(() => { flushed = true; });
  await f.reply(0); await first; assert.equal(flushed, false);
  await f.reply(1); await second; assert.equal(flushed, false);
  await f.reply(2); await third; await flushing; assert.equal(flushed, true); assert.equal(f.state().busy, false);
});
check('a later conflict cannot restart an earlier failed candidate', async () => {
  const f = fixture(), first = f.write('B', 'one'), second = f.write('C', 'two');
  await f.reply(0, 'offline'); await first; f.disk = 'external';
  await f.reply(1); assert.equal((await second).error.kind, 'clash');
  await f.advance(120_000); assert.equal(f.requests.length, 2); assert.equal(f.disk, 'external');
});

let failures = 0;
for (const [name, run] of cases) {
  try { await run(); console.log('PASS', name); }
  catch (error) { failures++; console.error('FAIL', name, '\n ', error.message); }
}
console.log(`save-road: ${cases.length - failures}/${cases.length} passed`);
if (failures) process.exitCode = 1;
