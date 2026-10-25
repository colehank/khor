import assert from 'node:assert/strict';
import {build} from 'esbuild';
import {fileURLToPath} from 'node:url';
const source = fileURLToPath(new URL('../src/lib/chat-drafts.ts', import.meta.url));
const compiled = await build({entryPoints: [source], bundle: true, write: false, platform: 'node', format: 'esm'});
const store = await import('data:text/javascript;base64,' + Buffer.from(compiled.outputFiles[0].text).toString('base64'));
const {chatDraftOwner: owner, retainCancelledChatDrafts: retain, rememberChatDraft: remember, completeChatDraft: complete, settleChatDraft: settle, recoverableChatDrafts: saved, discardChatDraft: discard, subscribeChatDrafts: subscribe} = store;
const draft = (session, token, text = 'same text') => ({owner: owner(session), session, token, text, full: text, images: [['image/png', 'synthetic base64']], documents: [['term.txt', 'block']], attachments: [{key: 'f1', name: 'tiny.png', size: 1, thumb: 'blob:already-owned', inline: ['image/png', 'synthetic base64'], path: '/fixture/tiny.png', error: null}]});
const settleCurrent = (session, token, reason) => settle(session, token, reason, owner(session));
const completeCurrent = (session, token) => complete(session, token, owner(session));
const discardCurrent = (session, token) => discard(session, token, owner(session));
let passed = 0;
function test(name, run) { retain([]); run(); passed++; console.log('PASS', name); }
test('snapshot shares large strings and drops revoked URL ownership', () => {
  retain(['alpha']); const input = draft('alpha', 'first'); const kept = remember(input);
  assert(kept); assert.notEqual(kept.attachments, input.attachments); assert.equal(kept.attachments[0].thumb, null);
  assert.equal(kept.images[0][1], input.images[0][1]); assert.notEqual(kept.attachments[0].inline, input.attachments[0].inline); assert.equal(kept.attachments[0].inline[1], input.attachments[0].inline[1]);
});
test('same text with different tokens settles only its original send', () => {
  retain(['alpha']); remember(draft('alpha', 'one')); remember(draft('alpha', 'two'));
  assert.equal(settleCurrent('alpha', 'one', 'cancelled').token, 'one');
  assert.equal(settleCurrent('alpha', 'one', 'cancelled'), null);
  assert.equal(settleCurrent('alpha', null, 'cancelled'), null);
  assert.equal(settleCurrent('alpha', 'two', 'failed').token, 'two');
  assert.deepEqual(saved('alpha').map(entry => entry.draft.token), ['one', 'two']);
});
test('commit frees pending and does not manufacture cancellation', () => {
  retain(['alpha']); remember(draft('alpha', 'committed')); completeCurrent('alpha', 'committed');
  assert.equal(settleCurrent('alpha', 'committed', 'cancelled'), null); assert.equal(saved('alpha').length, 0);
});
test('switching an open session keeps its snapshot and another session cannot consume it', () => {
  retain(['alpha', 'beta']); remember(draft('alpha', 'token')); remember(draft('beta', 'token', 'beta'));
  settleCurrent('alpha', 'token', 'cancelled'); retain(['alpha', 'beta']);
  assert.equal(saved('alpha').length, 1); assert.equal(saved('beta').length, 0);
  assert.equal(settleCurrent('beta', 'token', 'cancelled').text, 'beta');
});
test('close releases pending/saved data and refuses late registration or receipts', () => {
  retain(['alpha', 'beta']); const late = draft('alpha', 'late'); remember(late);
  remember(draft('alpha', 'saved')); settleCurrent('alpha', 'saved', 'cancelled'); retain(['beta']);
  assert.equal(saved('alpha').length, 0); assert.equal(remember(late), null); assert.equal(settleCurrent('alpha', 'late', 'cancelled'), null);
  retain(['alpha', 'beta']); assert.equal(settleCurrent('alpha', 'late', 'cancelled'), null, 'reopening does not resurrect discarded work');
  assert.equal(remember(late), null, 'a late original snapshot cannot register after reopen');
  const fresh = remember(draft('alpha', 'late')); assert(fresh);
  assert.equal(settle('alpha', 'late', 'cancelled', late.owner), null, 'old owner cannot consume the new owner even with the same token');
  assert.equal(settle('alpha', 'late', 'cancelled', fresh.owner).owner, fresh.owner);
});
test('saved snapshots have stable subscription identity and discard releases exactly one', () => {
  retain(['alpha']); let changes = 0; const stop = subscribe(() => changes++);
  const before = saved('alpha'); assert.equal(saved('alpha'), before);
  remember(draft('alpha', 'one')); assert.equal(changes, 0); settleCurrent('alpha', 'one', 'cancelled');
  const after = saved('alpha'); assert.notEqual(after, before); assert.equal(saved('alpha'), after); assert.equal(changes, 1);
  discardCurrent('alpha', 'unknown'); assert.equal(changes, 1); discardCurrent('alpha', 'one'); assert.equal(changes, 2); assert.equal(saved('alpha'), before);
  stop();
});
console.log(`chat drafts: ${passed}/${passed} passed`);
