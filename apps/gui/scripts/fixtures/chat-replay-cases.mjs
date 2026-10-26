import assert from 'node:assert/strict';

export const chatReplayCases = [
  'failed-empty', 'failed-partial', 'cancelled-delivery', 'empty-error',
  'successful-empty', 'failure-then-success', 'live-interleaved',
  'ordinary-control-error', 'successful-result', 'gone-reconnect', 'gone-flag-reconnect',
  'retry-mid-replay', 'successful-end-before-gone',
];

const message = (text, kind = 'history') => ({kind, update: {update: {
  sessionUpdate: 'agent_message_chunk', content: {type: 'text', text},
}}});
const end = {kind: 'history_end'};
const failure = (error = 'Replay cancelled', operation = 'replay') => ({
  kind: 'control_result', token: 'replay', operation, data: null, error,
});

export async function exerciseChatReplay(scenario, page, polls) {
  const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
  const until = async (check, what) => {
    const deadline = Date.now() + 8000;
    while (!await check()) { assert(Date.now() < deadline, what); await wait(10); }
  };
  let index = 0;
  const text = () => page.locator('[data-chat-column]').innerText();
  const check = async (present, absent = []) => {
    const drawn = await text();
    for (const word of present) assert(drawn.includes(word), `conversation retains ${word}: ${drawn}`);
    for (const word of absent) assert(!drawn.includes(word), `conversation excludes ${word}: ${drawn}`);
  };
  const deliver = async (frames, gone = false) => {
    const current = index++;
    await until(() => page.evaluate(i => window.__chatWire.pending.length > i, current), 'actual held poll is ready');
    await page.evaluate(({current, frames, gone}) => window.__chatWire.reply(current, {frames, gone, next: current + 1}), {current, frames, gone});
    await wait(20);
    await page.clock.runFor(0);
    await wait(20);
    if (!gone) await until(async () => (await polls()).length > index, 'actual poll completion schedules the next request');
  };
  const reconnect = async () => {
    const before = (await polls()).length;
    await page.evaluate(() => window.__chatReact.reload());
    await until(async () => (await polls()).length === before + 1, 'real tab action opens a new owner');
  };
  await deliver([message('Complete known history.'), end]);
  await check(['Complete known history.']);
  if (scenario === 'failed-empty') {
    await deliver([failure(), end]);
    await check(['Complete known history.']);
  } else if (scenario === 'failed-partial') {
    await deliver([message('Uncommitted fragment.')]);
    await check(['Complete known history.'], ['Uncommitted fragment.']);
    await deliver([failure()]);
    await deliver([message('Late failed fragment.'), end]);
    await check(['Complete known history.'], ['Uncommitted fragment.', 'Late failed fragment.']);
  } else if (scenario === 'cancelled-delivery') {
    await deliver([message('Uncommitted fragment.'), failure('Session history delivery was cancelled'), end]);
    await check(['Complete known history.'], ['Uncommitted fragment.']);
  } else if (scenario === 'empty-error') {
    await deliver([message('Uncommitted fragment.'), failure(''), end]);
    await check(['Complete known history.'], ['Uncommitted fragment.']);
  } else if (scenario === 'successful-empty') {
    await deliver([end]);
    await check([], ['Complete known history.']);
  } else if (scenario === 'failure-then-success') {
    await deliver([message('Failed fragment.'), failure(), end]);
    // A fresh replay must not inherit either the failure or partial frames.
    await deliver([message('Successful replacement.'), end]);
    await check(['Successful replacement.'], ['Complete known history.', 'Failed fragment.']);
    await deliver([end]);
    await check([], ['Successful replacement.']);
  } else if (scenario === 'live-interleaved') {
    await deliver([message('Failed fragment.'), message('Live before failure.', 'note'), failure()]);
    await deliver([message('Live after failure.', 'note'), end]);
    await check(['Complete known history.', 'Live before failure.', 'Live after failure.'], ['Failed fragment.']);
  } else if (scenario === 'ordinary-control-error') {
    await deliver([message('Successful replacement.'), failure('Mode denied', 'set_mode'), end]);
    await check(['Successful replacement.'], ['Complete known history.']);
  } else if (scenario === 'successful-result') {
    await deliver([message('Successful replacement.'), failure(null), end]);
    await check(['Successful replacement.'], ['Complete known history.']);
  } else if (scenario === 'gone-reconnect' || scenario === 'gone-flag-reconnect') {
    await deliver([message('Abandoned fragment.'), ...(scenario === 'gone-reconnect' ? [{kind: 'gone'}] : [])], true);
    await check(['Complete known history.'], ['Abandoned fragment.']);
    await reconnect();
    await deliver([message('New attachment history.'), end]);
    await check(['New attachment history.'], ['Complete known history.', 'Abandoned fragment.']);
  } else if (scenario === 'retry-mid-replay') {
    // Reload while an old wait is outstanding: its late reply must not paint.
    await deliver([message('Abandoned fragment.')]);
    const stale = index++;
    await reconnect();
    await deliver([message('New attachment history.'), end]);
    await page.evaluate(({stale, frame}) => window.__chatWire.reply(stale, {frames: [frame], gone: true}), {stale, frame: message('Late abandoned owner.')});
    await wait(20);
    await page.clock.runFor(0);
    assert.equal((await polls()).length, index + 1, 'late old-owner completion does not restart polling');
    await check(['New attachment history.'], ['Complete known history.', 'Abandoned fragment.', 'Late abandoned owner.']);
  } else if (scenario === 'successful-end-before-gone') {
    await deliver([message('Completed before disconnect.'), end, {kind: 'gone'}], true);
    await check(['Completed before disconnect.'], ['Complete known history.']);
  } else { throw new Error(`Missing replay case: ${scenario}`); }
}
