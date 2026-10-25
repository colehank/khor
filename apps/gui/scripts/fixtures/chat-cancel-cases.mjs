import assert from 'node:assert/strict';
export const extraCancelCases = ['new-draft-add', 'discard', 'multi-cancel', 'retry-token', 'web-admission', 'unconfirmed', 'unconfirmed-then-cancel', 'lost-commit', 'keyed-switch', 'app-switch-close', 'app-close-late', 'narrow', 'app-narrow', 'old-owner-gone', 'late-switch-cancel', 'restart-unknown'];
export async function exerciseCancellation(scenario, f) {
  const {page, draft, sent, session, emit, cancel, until, attach, evidence, engine} = f;
  const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
  const input = page.locator('[data-chat-input]');
  const rows = page.locator('[data-chat-cancelled-draft]');
  const sends = () => page.evaluate(() => window.__cancelWire.sends.map(send => send.args));
  const fresh = 'new text typed afterwards';
  const archive = async () => {
    await input.fill(fresh); await cancel(0);
    await until(async () => await rows.count() === 1, 'cancelled draft is retained alongside the new input');
    assert.equal(await input.inputValue(), fresh);
  };
  if (['new-draft-add', 'discard', 'multi-cancel', 'keyed-switch', 'app-switch-close', 'narrow', 'app-narrow'].includes(scenario)) await archive();
  if (scenario === 'new-draft-add') {
    await attach([{name: 'later.txt', mimeType: 'text/plain', buffer: Buffer.from('new selected file')}]);
    await page.locator('[data-chat-draft-add]').click();
    assert.equal(await input.inputValue(), `${fresh}\n\n${draft}`);
    assert.equal(await page.locator('[data-chat-pending-file]').count(), 3);
    assert.equal(await page.locator('[data-chat-doc]').count(), 1);
    await page.locator('[data-chat-send]').click(); await until(async () => (await sends()).length === 2, 'merged input sends');
    const second = (await sends())[1];
    assert.notEqual(second.token, sent.token); assert.deepEqual(second.images, sent.images); assert.deepEqual(second.files, sent.files);
    assert(second.text.includes('/fixture/inbox/later.txt'));
  } else if (scenario === 'discard') {
    await page.locator('[data-chat-draft-drop]').click(); assert.equal(await rows.count(), 0); assert.equal(await input.inputValue(), fresh);
    await emit([{kind: 'send_cancelled', token: sent.token, reconnect: false}]); await wait(80);
    assert.equal(await rows.count(), 0); assert.equal(await input.inputValue(), fresh);
  } else if (scenario === 'multi-cancel') {
    await page.locator('[data-chat-send]').click(); await until(async () => (await sends()).length === 2, 'second send entered');
    await input.fill('third draft'); await cancel(1);
    await until(async () => await rows.count() === 2, 'both independent cancelled snapshots survive');
    await page.locator('[data-chat-draft-add]').first().click();
    await page.locator('[data-chat-draft-add]').first().click();
    assert.equal(await input.inputValue(), `third draft\n\n${draft}\n\n${fresh}`);
    assert.equal(await page.locator('[data-chat-pending-file]').count(), 2); assert.equal(await page.locator('[data-chat-doc]').count(), 1);
    assert.equal(await rows.count(), 0);
  } else if (scenario === 'retry-token') {
    await emit([{kind: 'unsent', token: sent.token, miss: {kind: 'unreached', machine: null, raw: 'synthetic write failure'}}]);
    await until(async () => await rows.count() === 1, 'the failed token has its own recoverable snapshot');
    await page.evaluate(() => window.__cancelReact.retry());
    await until(async () => (await sends()).length === 2, 'production failure notice action retries');
    const second = (await sends())[1]; assert.notEqual(second.token, sent.token); assert.equal(second.text, sent.text);
    assert.deepEqual(second.images, sent.images); assert.deepEqual(second.files, sent.files);
    await emit([{kind: 'send_cancelled', token: sent.token, reconnect: false}, {kind: 'unsent', token: null, miss: {kind: 'other', machine: null, raw: 'old protocol failure'}}]);
    await wait(80); assert.equal(await input.inputValue(), ''); assert.equal(await page.locator('[data-chat-stop]').count(), 1);
  } else if (scenario === 'web-admission') {
    // Production api.ts has already received a successful HTTP null reply.
    await wait(100); await cancel(0);
    await until(async () => await input.inputValue() === draft, 'HTTP admission success must not release the pending snapshot');
    assert.equal(await page.locator('[data-chat-pending-file]').count(), 2);
  } else if (scenario === 'unconfirmed' || scenario === 'unconfirmed-then-cancel') {
    await page.evaluate(() => window.__cancelWire.sends[0].reject(new Error('synthetic RPC disconnected')));
    await until(async () => await page.locator('[data-reason=unconfirmed]').count() === 1, 'missing stable receipt becomes explicitly unconfirmed');
    assert.equal(await page.locator('[data-chat-draft-add]').count(), 0); assert.equal(await input.inputValue(), '');
    if (scenario === 'unconfirmed-then-cancel') {
      await cancel(0); await until(async () => await input.inputValue() === draft, 'a later stable cancellation resolves uncertainty and restores once');
      await wait(1100); assert.equal(await rows.count(), 0);
    } else {
      await page.locator('[data-chat-draft-drop]').click(); await cancel(0); await wait(80);
      assert.equal(await rows.count(), 0); assert.equal(await input.inputValue(), '', 'discarded recovery data cannot return on a late receipt');
    }
  } else if (scenario === 'lost-commit') {
    await page.evaluate(() => window.__cancelWire.sends[0].resolve(null));
    await emit([], false, {lost: true});
    await until(async () => await page.locator('[data-reason=unconfirmed]').count() === 1, 'a lost receipt is visible even after an early successful RPC');
    assert.equal(await page.locator('[data-chat-draft-add]').count(), 0);
    await emit([{kind: 'send_committed', token: sent.token}]);
    await until(async () => await rows.count() === 0, 'authoritative commit clears unknown recovery state');
    assert.equal(await input.inputValue(), '');
  } else if (scenario === 'keyed-switch') {
    await page.evaluate(() => window.__cancelReact.switch());
    await until(() => page.evaluate(() => window.__cancelWire.polls.some(p => !p.done && p.id === 'beta')), 'keyed beta component mounted');
    assert.equal(await rows.count(), 0);
    await page.evaluate(() => window.__cancelReact.back());
    await until(async () => await rows.count() === 1, 'alpha cancellation data survives an actual keyed unmount/remount');
    assert.equal(await input.inputValue(), fresh);
    await page.locator('[data-chat-draft-add]').click(); assert.equal(await input.inputValue(), `${fresh}\n\n${draft}`);
    assert.equal(await page.locator('[data-chat-pending-file]').count(), 2);
  } else if (scenario === 'narrow' || scenario === 'app-narrow') {
    await emit([{kind: 'note', update: {update: {sessionUpdate: 'agent_message_chunk', content: {type: 'text', text: 'Latest visible message stays above the composer.'}}}}]);
    await page.setViewportSize({width: 390, height: 844});
    // Actual App replaces its wide shell with its narrow shell. Wait for the
    // keyed view to remount and replay the transport's retained cursor data.
    await wait(500);
    await until(async () => {
      try { await page.locator('[data-chat-draft-add]').scrollIntoViewIfNeeded({timeout: 1000}); return true; }
      catch { return false; }
    }, 'the narrow shell settled on its recovery entry');
    await until(async () => (await page.locator('body').innerText()).includes('Latest visible message stays above the composer.'), 'the remounted renderer replayed its latest message');
    await page.screenshot({path: `${evidence}/${engine}-${scenario}-cancelled-draft.png`});
    assert(await input.isVisible());
    assert.equal(await input.inputValue(), fresh);
    f.layout = await page.evaluate(() => ({input: document.querySelector('[data-chat-input]').getBoundingClientRect().toJSON(), row: document.querySelector('[data-chat-cancelled-draft]').getBoundingClientRect().toJSON(), width: innerWidth, height: innerHeight, scrollWidth: document.documentElement.scrollWidth}));
  } else if (scenario === 'app-switch-close') {
    await page.locator('[data-tab-open="gui/ai-elements-plan"]').click();
    await until(() => page.evaluate(() => window.__cancelWire.polls.some(p => !p.done && p.id === 'gui/ai-elements-plan')), 'actual App switched to its second session');
    assert.equal(await rows.count(), 0);
    await page.locator(`[data-tab-open="${session}"]`).click();
    await until(async () => await rows.count() === 1, 'actual App preserves the original session recovery data');
    assert.equal(await input.inputValue(), fresh);
    await page.evaluate(async id => { const m = await import('/src/lib/chat-drafts.ts'); window.__savedBeforeClose = m.recoverableChatDrafts(id)[0].draft; }, session);
    await page.locator(`[data-tab="${session}"] [data-tab-close]`).click();
    await until(async () => await page.locator(`[data-tab="${session}"]`).count() === 0, 'actual App closed the tab');
    const closed = await page.evaluate(async id => { const m = await import('/src/lib/chat-drafts.ts'); return {count: m.recoverableChatDrafts(id).length, late: m.rememberChatDraft(window.__savedBeforeClose)}; }, session);
    assert.deepEqual(closed, {count: 0, late: null});
    await page.locator('[data-row-open]').filter({hasText: 'AI Elements · 组件总览'}).first().click();
    await until(async () => await page.locator(`[data-tab="${session}"]`).count() === 1, 'actual App reopened the same session id');
    assert.equal(await rows.count(), 0);
    assert.equal(await page.evaluate(async () => (await import('/src/lib/chat-drafts.ts')).rememberChatDraft(window.__savedBeforeClose)), null, 'the old closed-tab capability stays revoked after reopen');
  } else if (scenario === 'restart-unknown') {
    await page.evaluate(() => { window.__cancelWire.sends[0].resolve(null); window.__cancelWire.freshNextOpen = true; window.__cancelReact.reload(); });
    await until(async () => await page.locator('[data-reason=unconfirmed]').count() === 1, 'fresh attachment marks its older missing receipt unknown');
    await input.fill('new message after reconnect');
    assert.equal(await page.locator('[data-chat-stop]').count(), 0, 'the old upload busy flag does not survive a fresh attachment');
    await page.locator('[data-chat-send]').click();
    await until(async () => (await sends()).length === 2, 'the fresh attachment can send a new draft');
    assert.notEqual((await sends())[1].token, sent.token);
  } else if (scenario === 'late-switch-cancel') {
    await page.evaluate(() => window.__cancelReact.switch());
    await until(() => page.evaluate(() => window.__cancelWire.polls.some(p => !p.done && p.id === 'beta')), 'beta view mounted');
    await page.evaluate(() => window.__cancelReact.back());
    await until(() => page.evaluate(() => window.__cancelWire.polls.filter(p => !p.done && p.id === 'alpha').length >= 2), 'alpha mounted its own new reader');
    await cancel(0);
    await until(async () => await input.inputValue() === draft, 'late original cancellation is restored by the current empty same-session composer');
    assert.equal(await page.locator('[data-chat-pending-file]').count(), 2); assert.equal(await page.locator('[data-chat-doc]').count(), 1);
  } else if (scenario === 'old-owner-gone') {
    await page.evaluate(() => window.__cancelReact.switch());
    await until(() => page.evaluate(() => window.__cancelWire.polls.some(p => !p.done && p.id === 'beta')), 'keyed beta view mounted');
    await page.evaluate(() => window.__cancelReact.back());
    await input.fill('new active turn after remount');
    await page.locator('[data-chat-send]').click();
    await until(async () => (await sends()).length === 2, 'new alpha owner sent another turn');
    const newer = (await sends())[1];
    await page.evaluate(() => {
      const old = window.__cancelWire.polls.find(p => p.id === 'alpha' && !p.done);
      old.done = true;
      old.resolve({frames: [], next: 1, gone: true, lost: false, sending: 0, sending_of: 0, stall: null});
    });
    await wait(100);
    const statuses = await page.evaluate(async id => (await import('/src/lib/chat-drafts.ts')).recoverableChatDrafts(id).map(entry => ({token: entry.draft.token, reason: entry.reason})), session);
    assert(!statuses.some(entry => entry.token === newer.token), 'an old view gone reply cannot mark a newer attachment send unconfirmed');
    assert.equal(await page.locator('[data-chat-stop]').count(), 1);
  } else if (scenario === 'app-close-late') {
    await page.locator(`[data-tab="${session}"] [data-tab-close]`).click();
    await until(async () => await page.locator(`[data-tab="${session}"]`).count() === 0, 'actual App closed the active pending send');
    await cancel(0); await wait(100);
    const count = await page.evaluate(async id => (await import('/src/lib/chat-drafts.ts')).recoverableChatDrafts(id).length, session);
    assert.equal(count, 0, 'late cancelled receipt does not resurrect a closed tab');
    assert.equal(await rows.count(), 0);
  } else { throw new Error(`No cancellation case: ${scenario}`); }
}
