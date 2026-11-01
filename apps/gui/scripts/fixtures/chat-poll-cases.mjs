import assert from 'node:assert/strict';
export const chatPollCases = ['healthy-wait', 'success-resets', 'backoff-cap', 'retry-attempt', 'gone-reload', 'unmount-inflight', 'switch-owner', 'visibility-backoff', 'http-failure'];
export async function exerciseChatPoll(scenario, page, polls) {
  const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
  const until = async (check, what) => {
    const end = Date.now() + 8000;
    while (!await check()) { assert(Date.now() < end, what); await wait(10); }
  };
  const advance = async ms => { await page.clock.runFor(ms); await wait(10); };
  const count = async () => (await polls()).length;
  const mode = value => page.evaluate(value => { window.__chatWire.mode = value; }, value);
  const reply = (index, change = {}) => page.evaluate(({index, change}) => window.__chatWire.reply(index, change), {index, change});
  const reject = index => page.evaluate(index => window.__chatWire.reject(index), index);
  const commandCount = cmd => page.evaluate(cmd => window.__chatWire.calls.filter(call => call.cmd === cmd).length, cmd);
  // Poll owns live controls; conversation bodies arrive through history pages.
  const message = text => ({kind: 'ask', ask: 1, title: text, options: [['ack', 'Acknowledge']]});
  const nextPoll = async before => { await advance(0); await until(async () => await count() === before + 1, 'completion starts the next held wait'); };
  if (scenario === 'http-failure') {
    for (const delay of [500, 500, 500, 800]) {
      const before = await count(); await advance(delay - 1);
      assert.equal(await count(), before, 'HTTP polling keeps its pace floor through failures');
      await advance(1); assert.equal(await count(), before + 1);
    }
    const calls = await polls();
    assert(calls.every(call => call.cmd === 'chat_poll'), 'the actual web API path uses poll rather than native wait');
  } else if (scenario === 'healthy-wait') {
    await advance(1000); assert.equal(await count(), 1, 'only one held wait is in flight');
    await reply(0, {next: 7, frames: [message('fresh healthy frame')]}); await nextPoll(1);
    const calls = await polls();
    assert.equal(calls[1].at - calls[0].at, 1000, 'healthy held completion adds no timer beat');
    assert.equal(calls[1].args.since, 7, 'the next request carries the completed cursor');
    await until(async () => (await page.locator('body').innerText()).includes('fresh healthy frame'), 'real renderer painted the message');
    await advance(2000); assert.equal(await count(), 2, 'a pending second wait does not fork');
  } else if (scenario === 'success-resets') {
    await advance(100); assert.equal(await count(), 2);
    await mode('hold'); await advance(200); assert.equal(await count(), 3);
    await mode('fail'); await reply(0, {next: 9}); await nextPoll(3);
    await advance(90); assert.equal(await count(), 4);
    await advance(10); assert.equal(await count(), 5, 'success resets the next failure delay to 100ms');
    assert.equal((await polls())[4].args.since, 9);
  } else if (scenario === 'backoff-cap') {
    for (const delay of [100, 200, 400, 800, 1600, 3200, 5000, 5000]) {
      const before = await count(); await advance(delay - 1);
      assert.equal(await count(), before, `does not retry before ${delay}ms`);
      await advance(1); assert.equal(await count(), before + 1, `retries after ${delay}ms`);
    }
    const times = (await polls()).map(call => call.at);
    assert.deepEqual(times.slice(1).map((at, i) => at - times[i]), [100, 200, 400, 800, 1600, 3200, 5000, 5000]);
  } else if (scenario === 'retry-attempt') {
    await advance(100); assert.equal(await count(), 2);
    await page.evaluate(() => window.__chatReact.reload());
    await until(async () => await count() === 3, 'actual tab reload starts a new attempt');
    assert.equal(await commandCount('chat_open'), 2);
    await advance(99); assert.equal(await count(), 3);
    await advance(1); assert.equal(await count(), 4, 'new attempt resets backoff and cancels the old timer');
  } else if (scenario === 'gone-reload') {
    await advance(1000); assert.equal(await count(), 1);
    await mode('hold'); await page.evaluate(() => window.__chatReact.reload());
    await until(async () => await count() === 2, 'a new attempt reopens a finished attachment');
    assert.equal(await commandCount('chat_open'), 2);
    await reply(0, {next: 2}); await nextPoll(2);
  } else if (scenario === 'unmount-inflight') {
    await page.evaluate(() => window.__chatReact.close());
    await until(async () => await commandCount('chat_leave') === 1, 'unmount releases the attachment');
    await reply(0, {gone: true, lost: true, frames: [message('late after unmount')]});
    await advance(60_000);
    assert.equal(await count(), 1); assert.equal(await commandCount('chat_replay'), 0);
    assert(!(await page.locator('body').innerText()).includes('late after unmount'));
    await page.evaluate(() => window.__chatReact.open());
    await until(async () => await count() === 2, 'fixture remounted actual component');
    await page.evaluate(() => window.__chatReact.close());
    await until(async () => await commandCount('chat_leave') === 2, 'remount also cleaned up');
    await reject(1); await advance(60_000); assert.equal(await count(), 2, 'late failure after unmount never retries');
  } else if (scenario === 'switch-owner') {
    await page.evaluate(() => window.__chatReact.switch());
    await until(async () => (await polls()).some(call => call.args.id === 'beta'), 'new owner opened and asked');
    await reply(1, {next: 4, frames: [message('fresh beta frame')]}); await nextPoll(2);
    await until(async () => (await page.locator('body').innerText()).includes('fresh beta frame'), 'new owner paints its own frame');
    await reply(0, {next: 99, gone: true, lost: true, frames: [message('stale alpha frame')]});
    await advance(5000);
    assert.equal((await polls()).filter(call => call.args.id === 'alpha').length, 1);
    assert.equal(await commandCount('chat_replay'), 0, 'stale lost frames cannot request old-owner replay');
    const text = await page.locator('body').innerText();
    assert(text.includes('fresh beta frame')); assert(!text.includes('stale alpha frame'));
  } else if (scenario === 'visibility-backoff') {
    await page.evaluate(() => window.__chatWire.visibility(true));
    await reject(0); await advance(9999); assert.equal(await count(), 1, 'hidden failure keeps the 10s background floor');
    await advance(1); assert.equal(await count(), 2);
    assert.equal((await polls())[1].cmd, 'chat_poll');
    await reject(1);
    await page.evaluate(() => { window.__chatWire.visibility(false); window.__chatWire.visibility(true); window.__chatWire.visibility(false); });
    await advance(199); assert.equal(await count(), 2, 'visibility kicks cannot bypass the outstanding 200ms failure delay');
    await advance(1); assert.equal(await count(), 3, 'restoring visibility sheds the background floor after failure backoff');
  } else { throw new Error(`No chat poll case: ${scenario}`); }
}
