import assert from 'node:assert/strict';
import {build} from 'esbuild';
import {fileURLToPath} from 'node:url';
const source = fileURLToPath(new URL('../src/lib/chat-drafts.ts', import.meta.url));
const compiled = await build({entryPoints: [source], bundle: true, write: false, platform: 'node', format: 'esm'});
const store = await import('data:text/javascript;base64,' + Buffer.from(compiled.outputFiles[0].text).toString('base64'));
const {awaitChatDraftReceipt: awaitReceipt, unconfirmedChatDrafts: unknownAll, chatDraftOwner: owner, retainCancelledChatDrafts: retain, rememberChatDraft: remember, completeChatDraft: complete, settleChatDraft: settle, recoverableChatDrafts: saved, discardChatDraft: discard, subscribeChatDrafts: subscribe} = store;
const draft = (session, token, text = 'same text') => ({owner: owner(session), reader: {}, session, token, text, full: text, images: [['image/png', 'synthetic base64']], documents: [['term.txt', 'block']], attachments: [{key: 'f1', name: 'tiny.png', size: 1, thumb: 'blob:already-owned', inline: ['image/png', 'synthetic base64'], path: '/fixture/tiny.png', error: null}]});
const settleCurrent = (session, token, reason) => settle(session, token, reason, owner(session));
const completeCurrent = (session, token) => complete(session, token, owner(session));
const discardCurrent = (session, token) => discard(session, token, owner(session));
let passed = 0;
async function test(name, run) { retain([]); await run(); passed++; console.log('PASS', name); }
await test('snapshot shares large strings and drops revoked URL ownership', () => {
  retain(['alpha']); const input = draft('alpha', 'first'); const kept = remember(input);
  assert(kept); assert.notEqual(kept.attachments, input.attachments); assert.equal(kept.attachments[0].thumb, null);
  assert.equal(kept.images[0][1], input.images[0][1]); assert.notEqual(kept.attachments[0].inline, input.attachments[0].inline); assert.equal(kept.attachments[0].inline[1], input.attachments[0].inline[1]);
});
await test('same text with different tokens settles only its original send', () => {
  retain(['alpha']); remember(draft('alpha', 'one')); remember(draft('alpha', 'two'));
  assert.equal(settleCurrent('alpha', 'one', 'cancelled').token, 'one');
  assert.equal(settleCurrent('alpha', 'one', 'cancelled'), null);
  assert.equal(settleCurrent('alpha', null, 'cancelled'), null);
  assert.equal(settleCurrent('alpha', 'two', 'failed').token, 'two');
  assert.deepEqual(saved('alpha').map(entry => entry.draft.token), ['one', 'two']);
});
await test('commit frees pending and does not manufacture cancellation', () => {
  retain(['alpha']); remember(draft('alpha', 'committed')); completeCurrent('alpha', 'committed');
  assert.equal(settleCurrent('alpha', 'committed', 'cancelled'), null); assert.equal(saved('alpha').length, 0);
});
await test('switching an open session keeps its snapshot and another session cannot consume it', () => {
  retain(['alpha', 'beta']); remember(draft('alpha', 'token')); remember(draft('beta', 'token', 'beta'));
  settleCurrent('alpha', 'token', 'cancelled'); retain(['alpha', 'beta']);
  assert.equal(saved('alpha').length, 1); assert.equal(saved('beta').length, 0);
  assert.equal(settleCurrent('beta', 'token', 'cancelled').text, 'beta');
});
await test('close releases pending/saved data and refuses late registration or receipts', () => {
  retain(['alpha', 'beta']); const late = draft('alpha', 'late'); remember(late);
  remember(draft('alpha', 'saved')); settleCurrent('alpha', 'saved', 'cancelled'); retain(['beta']);
  assert.equal(saved('alpha').length, 0); assert.equal(remember(late), null); assert.equal(settleCurrent('alpha', 'late', 'cancelled'), null);
  retain(['alpha', 'beta']); assert.equal(settleCurrent('alpha', 'late', 'cancelled'), null, 'reopening does not resurrect discarded work');
  assert.equal(remember(late), null, 'a late original snapshot cannot register after reopen');
  const fresh = remember(draft('alpha', 'late')); assert(fresh);
  assert.equal(settle('alpha', 'late', 'cancelled', late.owner), null, 'old owner cannot consume the new owner even with the same token');
  assert.equal(settle('alpha', 'late', 'cancelled', fresh.owner).owner, fresh.owner);
});
await test('saved snapshots have stable subscription identity and discard releases exactly one', () => {
  retain(['alpha']); let changes = 0; const stop = subscribe(() => changes++);
  const before = saved('alpha'); assert.equal(saved('alpha'), before);
  remember(draft('alpha', 'one')); assert.equal(changes, 0); settleCurrent('alpha', 'one', 'cancelled');
  const after = saved('alpha'); assert.notEqual(after, before); assert.equal(saved('alpha'), after); assert.equal(changes, 1);
  discardCurrent('alpha', 'unknown'); assert.equal(changes, 1); discardCurrent('alpha', 'one'); assert.equal(changes, 2); assert.equal(saved('alpha'), before);
  stop();
});
await test('a rejected RPC without a receipt becomes explicitly unconfirmed after one second', async () => {
  retain(['alpha']); const item = remember(draft('alpha', 'unknown'));
  awaitReceipt('alpha', item.token, item.owner);
  assert.equal(saved('alpha').length, 0);
  await new Promise(resolve => setTimeout(resolve, 1040));
  assert.equal(saved('alpha')[0].reason, 'unconfirmed');
  assert.equal(settle('alpha', item.token, 'cancelled', item.owner), item);
  assert.equal(saved('alpha')[0].reason, 'cancelled'); assert.equal(saved('alpha').length, 1);
});
await test('commit/cancel/failure receipts invalidate rejection timers', async () => {
  retain(['alpha']);
  for (const token of ['commit', 'cancel', 'failure']) {
    const item = remember(draft('alpha', token)); awaitReceipt('alpha', token, item.owner, 10);
    if (token === 'commit') complete('alpha', token, item.owner);
    else settle('alpha', token, token === 'cancel' ? 'cancelled' : 'failed', item.owner);
  }
  await new Promise(resolve => setTimeout(resolve, 30));
  assert.deepEqual(saved('alpha').map(entry => entry.reason), ['cancelled', 'failed']);
});
await test('close/reopen expires old timers even if a new owner reuses the token', async () => {
  retain(['alpha']); const old = remember(draft('alpha', 'reused')); awaitReceipt('alpha', old.token, old.owner, 10);
  retain([]); retain(['alpha']); remember(draft('alpha', 'reused'));
  awaitReceipt('alpha', old.token, old.owner, 10);
  await new Promise(resolve => setTimeout(resolve, 30)); assert.equal(saved('alpha').length, 0);
});
await test('lost/gone marks only outstanding original-session sends unknown and commit clears them', () => {
  retain(['alpha', 'beta']); const first = remember(draft('alpha', 'one')); remember(draft('alpha', 'two')); remember(draft('beta', 'one'));
  unknownAll('alpha', first.owner); unknownAll('alpha', first.owner);
  assert.equal(saved('alpha').length, 2); assert.equal(saved('beta').length, 0);
  complete('alpha', 'one', first.owner); assert.deepEqual(saved('alpha').map(entry => entry.draft.token), ['two']);
  discard('alpha', 'two', first.owner); assert.equal(settle('alpha', 'two', 'cancelled', first.owner), null);
});
await test('an old stopped reader cannot settle or mark a newer attachment send unknown', () => {
  retain(['alpha']); const oldReader = {}, newReader = {};
  const old = remember({...draft('alpha', 'old'), reader: oldReader});
  const current = remember({...draft('alpha', 'new'), reader: newReader});
  unknownAll('alpha', old.owner, {reader: oldReader});
  assert.deepEqual(saved('alpha').map(entry => entry.draft.token), ['old']);
  assert.equal(settle('alpha', current.token, 'cancelled', old.owner, oldReader), null);
  complete('alpha', current.token, old.owner, oldReader);
  assert.equal(settle('alpha', current.token, 'cancelled', current.owner, newReader), current);
});
await test('a fresh attachment marks only older readers uncertain', () => {
  retain(['alpha']); const previous = remember(draft('alpha', 'previous')), current = remember(draft('alpha', 'current'));
  unknownAll('alpha', current.owner, {reader: current.reader, except: true});
  assert.deepEqual(saved('alpha').map(entry => entry.draft.token), [previous.token]);
});
retain([]);
console.log(`chat drafts: ${passed}/${passed} passed`);
