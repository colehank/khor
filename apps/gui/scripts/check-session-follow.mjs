import assert from 'node:assert/strict';
import { build } from 'esbuild';
import { fileURLToPath } from 'node:url';
const here = fileURLToPath(new URL('.', import.meta.url));
const entry = new URL('../src/lib/deck-joystick.ts', import.meta.url);
const { outputFiles } = await build({ entryPoints: [fileURLToPath(entry)], bundle: true, format: 'esm', write: false, absWorkingDir: here });
const { deckJoystick, DECK_JOYSTICK_DEFAULTS } = await import('data:text/javascript;base64,' + Buffer.from(outputFiles[0].text).toString('base64'));
let passed = 0;
function test(name, run) { run(); passed++; console.log(`PASS ${name}`); }
function fixture(count = 8, initial = 3, held = true) {
  let now = 0, id = 0, overviews = 0;
  const jobs = new Map(), commits = [], samples = [];
  const clock = { now: () => now, request: run => { jobs.set(++id, { at: now + 16, run }); return id; }, cancel: id => jobs.delete(id) };
  const engine = deckJoystick(count, initial, s => samples.push(s), i => commits.push(i), () => overviews++, clock);
  const advance = ms => {
    const end = now + ms;
    for (;;) {
      const next = [...jobs].find(([, job]) => job.at <= end);
      if (!next) break;
      now = next[1].at; jobs.delete(next[0]); next[1].run();
    }
    now = end;
  };
  engine.start(200, 500);
  if (held) advance(DECK_JOYSTICK_DEFAULTS.holdMs + 16);
  return { engine, advance, move: (dx, dy = 0) => engine.move(200 + dx, 500 + dy), commits, jobs, samples, initial, overviews: () => overviews };
}
for (const sign of [-1, 1]) test(`follow and threshold, sign ${sign}`, () => {
  const f = fixture();
  assert.equal(f.engine.read().preview, 3);
  f.move(sign * 12);
  const first = f.engine.read();
  assert.equal(first.preview, 3, 'small motion only moves the candidates; it does not select another identity');
  assert(first.position !== 3 && Math.sign(first.position - 3) === -sign, 'continuous position follows the finger visually');
  f.move(sign * 24); const next = f.engine.read();
  assert(Math.abs(next.position - 3) > Math.abs(first.position - 3));
  f.move(sign * 31); const before = f.engine.read().position;
  f.move(sign * 32); const crossed = f.engine.read();
  assert.equal(crossed.target, 3 - sign);
  assert(Math.abs(crossed.position - before) <= 1 / 64 + 1e-8, 'crossing threshold cannot jump the presentation');
  f.advance(80); const middle = f.engine.read().position;
  assert((middle - crossed.position) * -sign > 0 && (crossed.target - middle) * -sign > 0);
  f.advance(96); assert.equal(f.engine.read().position, 3 - sign);
  assert.deepEqual(f.commits, []); f.engine.release(); f.engine.release();
  assert.deepEqual(f.commits, [3 - sign]); assert.equal(f.jobs.size, 0);
});
test('subthreshold return and release leave the session unchanged', () => {
  const f = fixture(); f.move(24); f.move(12); f.move(0);
  assert.equal(f.engine.read().position, 3); f.advance(5000);
  assert.equal(f.engine.release(), 'none'); assert.deepEqual(f.commits, []); assert.equal(f.jobs.size, 0);
});
test('return to centre pauses completed selection and reverse follows before snapping', () => {
  const f = fixture(); f.move(32); f.advance(176); f.move(0);
  const target = f.engine.read().target; f.advance(6000);
  assert.equal(f.engine.read().target, target); assert.equal(f.engine.read().direction, 0);
  f.move(-16); const small = f.engine.read();
  assert.equal(small.target, target); assert(small.position > target && small.position < target + 1);
  f.move(-50); f.advance(176); assert.equal(f.engine.read().target, target + 1);
  assert.deepEqual(f.commits, []); f.engine.cancel();
});
test('coalesced crossing brakes first; a stationary opposite event cannot reverse', () => {
  const f = fixture(); f.move(40); f.advance(176); f.move(-50);
  const target = f.engine.read().target; f.move(-50); f.advance(5000);
  assert.equal(f.engine.read().target, target); assert.equal(f.engine.read().direction, 0);
  f.move(-58); assert.equal(f.engine.read().target, target);
  assert(f.engine.read().position > target && f.engine.read().position < target + 1);
  f.move(-90); f.advance(176); assert.equal(f.engine.read().target, target + 1); f.engine.cancel();
});
test('reverse during snap resumes from the visible position', () => {
  const f = fixture(); f.move(32); f.advance(48); f.move(-50);
  const before = f.engine.read().position; f.move(-58); const after = f.engine.read().position;
  assert(after >= before && after - before <= 8 / 64 + 1e-8, 'no reset to an integer or to the gesture origin');
  f.engine.cancel();
  const g = fixture(); g.move(32); g.advance(176);
  while (g.engine.read().target === 2) g.advance(16);
  g.move(-50); g.move(-58); g.move(-90);
  const reversed = g.engine.read();
  assert(reversed.position <= reversed.target, 'reverse cannot overshoot then recoil toward its accepted candidate');
  g.advance(1800); assert(g.engine.read().target > reversed.target, 'an already aligned snap still schedules its next step');
  g.engine.cancel();
});
for (const count of [2, 3, 8]) test(`auto continues through cyclic seam, ${count} identities`, () => {
  const f = fixture(count, 0); f.move(40); const first = f.engine.read();
  assert.equal(first.preview, count - 1); f.advance(18000);
  const before = f.engine.read(); assert(before.position < -count && before.target < -count);
  const motion = f.samples.filter(s => s.phase === 'steering').map(s => s.position);
  assert(motion.every((x, i) => i === 0 || x <= motion[i - 1] + 1e-8), 'no backwards wrap, integer reset or recoil');
  assert.deepEqual(f.commits, []); f.engine.release();
  assert.deepEqual(f.commits, before.preview === 0 ? [] : [before.preview]); assert.equal(f.jobs.size, 0);
});
test('snap target is selected once even when released before animation completes', () => {
  const f = fixture(); f.move(32); assert.notEqual(f.engine.read().position, f.engine.read().target);
  assert.equal(f.engine.release(), 'commit'); f.advance(5000); f.engine.release();
  assert.deepEqual(f.commits, [2]); assert.equal(f.jobs.size, 0);
});
test('threshold jitter cannot skip another candidate', () => {
  const f = fixture(); f.move(32);
  for (const dx of [30, 33, 31, 34, 30, 32]) { f.move(dx); f.advance(16); }
  assert.equal(f.engine.read().pulse, 1); assert.equal(f.engine.read().target, 2); f.engine.cancel();
});
test('pause and a zero move preserve target during overview/home round trip', () => {
  const f = fixture(); f.move(-32); f.advance(48); const target = f.engine.read().target;
  f.engine.pause(); f.move(0); f.advance(4000); assert.equal(f.engine.read().target, target);
  assert.equal(f.engine.read().position, target); assert.equal(f.engine.read().direction, 0); f.engine.cancel();
});
test('hold rhythm only starts after first threshold; no stationary subthreshold advance', () => {
  const f = fixture(); f.move(24); f.advance(6000); assert.equal(f.engine.read().target, 3);
  f.move(32); f.advance(176); const landed = f.engine.read().target; f.advance(1200);
  assert.equal(f.engine.read().target, landed); f.advance(240); assert.equal(f.engine.read().target, landed - 1); f.engine.cancel();
});
test('pre-hold swipes stay routed to ordinary swipe recognizer', () => {
  const f = fixture(8, 3, false); f.move(20); assert.equal(f.engine.read().phase, 'swipe');
  f.advance(1000); assert.equal(f.engine.read().phase, 'swipe'); assert.equal(f.engine.read().position, 3);
  assert.equal(f.engine.release(), 'none'); assert.deepEqual(f.commits, []);
});
test('upward overview and short tap preserve their contracts', () => {
  const f = fixture(8, 3, false); assert.equal(f.engine.release(), 'tap');
  f.engine.start(200, 500); f.move(0, -60); assert.equal(f.engine.read().phase, 'pull');
  assert.equal(f.engine.release(), 'overview'); assert.equal(f.overviews(), 1); assert.deepEqual(f.commits, []);
});
for (const count of [0, 1]) test(`empty/single collection stays still (${count})`, () => {
  const f = fixture(count, 0); f.move(80); f.advance(8000); assert.equal(f.engine.read().position, 0);
  f.engine.release(); assert.deepEqual(f.commits, []); assert.equal(f.jobs.size, 0);
});
test('cancel and dispose clear every scheduled callback', () => {
  for (const method of ['cancel', 'dispose']) {
    const f = fixture(); f.move(50); f.advance(80); f.engine[method]();
    const count = f.samples.length; f.advance(5000); assert.equal(f.samples.length, count);
    assert.equal(f.jobs.size, 0); assert.deepEqual(f.commits, []);
  }
});
console.log(`${passed} continuous-follow behavior checks passed.`);
