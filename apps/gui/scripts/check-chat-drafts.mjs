import {SessionContentDock} from '../src/lib/session-content-dock.ts';
import {ReadFreshness} from '../src/lib/read-freshness.ts';
import {ComposerFiles,composerReady} from '../src/lib/composer-files.ts';
import assert from 'node:assert/strict';
import {build} from 'esbuild';
import {fileURLToPath} from 'node:url';
import {SessionDraftStore} from '../src/lib/session-drafts.ts';
import {RestartChecklist} from '../src/lib/restart-checklist.ts';
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
await test('update preparation reads live state, never saves a file, and releases closed owners', () => {
  const checklist = new RestartChecklist();
  let unsaved = true;
  const leave = checklist.register(() => unsaved ? 'unsaved-files' : null);
  assert.deepEqual(checklist.prepare(), ['unsaved-files']);
  unsaved = false;
  assert.deepEqual(checklist.prepare(), []);
  unsaved = true;
  leave();
  assert.deepEqual(checklist.prepare(), []);
});
await test('update preparation fails closed on checkpoint failure and retries current state', () => {
  const checklist = new RestartChecklist();
  let writable = false;
  checklist.register(() => { if (!writable) throw new Error('storage unavailable'); return null; });
  checklist.register(() => writable ? null : 'draft-storage');
  assert.deepEqual(checklist.prepare(), ['draft-storage']);
  writable = true;
  assert.deepEqual(checklist.prepare(), []);
});
await test('an unconfirmed or recoverable send blocks restart until its actual receipt or discard', () => {
  retain(['alpha']);
  assert.equal(store.chatDraftsAtRisk(), false);
  remember(draft('alpha', 'pending'));
  assert.equal(store.chatDraftsAtRisk(), true);
  completeCurrent('alpha', 'pending');
  assert.equal(store.chatDraftsAtRisk(), false);
  remember(draft('alpha', 'failed'));
  settleCurrent('alpha', 'failed', 'failed');
  assert.equal(store.chatDraftsAtRisk(), true);
  discardCurrent('alpha', 'failed');
  assert.equal(store.chatDraftsAtRisk(), false);
});
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
await test('unsent draft checkpoint refuses quota loss without evicting another session', () => {
  const values=new Map();let full=false;
  const storage={getItem:key=>values.get(key)??null,setItem:(key,value)=>{if(full)throw new Error('quota');values.set(key,value);}};
  const drafts=new SessionDraftStore(storage);
  drafts.set('khor:alpha','中文 draft A');full=true;drafts.set('gui:beta','draft B');
  assert.equal(drafts.checkpoint(),false,'an updater must not restart over an uncommitted draft');
  assert.equal(drafts.get('gui:beta'),'draft B','typing remains available after storage failure');
  full=false;assert.equal(drafts.checkpoint(),true);
  const reopened=new SessionDraftStore(storage);assert.equal(reopened.get('khor:alpha'),'中文 draft A');assert.equal(reopened.get('gui:beta'),'draft B');
  drafts.set('khor:alpha','large'.repeat(150000));assert.equal(drafts.checkpoint(),false);
  assert.equal(drafts.get('gui:beta'),'draft B','capacity limits never evict an unrelated live draft');
  reopened.retain(['beta']);assert.equal(new SessionDraftStore(storage).get('khor:alpha'),'','closed tabs do not resurrect drafts');
  const damaged=new Map([['khor.session-drafts.v1','{broken']]);
  const corrupt=new SessionDraftStore({getItem:key=>damaged.get(key)??null,setItem:(key,value)=>damaged.set(key,value)});
  corrupt.set('gui:new','new input');assert.equal(corrupt.checkpoint(),false);assert.equal(damaged.get('khor.session-drafts.v1'),'{broken');
});
retain([]);
console.log(`chat drafts: ${passed}/${passed} passed`);

await test('composer keeps uploads with their session through detach and resolves only existing picks',()=>{
 const revoked=[],a=new ComposerFiles(url=>revoked.push(url)),b=new ComposerFiles();
 const key=a.key();a.add([{key,name:'photo.png',size:20,thumb:'blob:owned',inline:null,path:null,error:null,encoding:true}]);
 a.patch(key,{path:'/receive/photo.png'});assert.equal(composerReady(a.snapshot()),false);
 a.patch(key,{inline:['image/png','bytes'],encoding:false});assert.equal(composerReady(a.snapshot()),true);
 assert.equal(b.snapshot().length,0);assert.equal(a.snapshot()[0].thumb,'blob:owned');
 a.remove(key);assert.deepEqual(revoked,['blob:owned']);a.patch(key,{path:'late response'});assert.equal(a.snapshot().length,0);
});
await test('failed uploads and encoding cannot be silently filtered out of a send',()=>{
 const files=new ComposerFiles();const key=files.key();
 files.add([{key,name:'failed.pdf',size:null,thumb:null,inline:null,path:null,error:'offline'}]);
 assert.equal(composerReady(files.snapshot()),false);
 files.patch(key,{path:'/receive/file.pdf'});assert.equal(composerReady(files.snapshot()),false);
 files.remove(key);assert.equal(composerReady(files.snapshot()),true);
});
await test('clear releases every owned preview and a late encode cannot repopulate sent draft',()=>{
 const revoked=[],files=new ComposerFiles(url=>revoked.push(url)),key=files.key();
 files.add([{key,name:'a.png',size:1,thumb:'blob:a',inline:null,path:'/a',error:null}]);
 files.clear();files.patch(key,{inline:['image/png','late']});assert.equal(files.snapshot().length,0);assert.deepEqual(revoked,['blob:a']);
});

await test('read failures expose an unconfirmed connection and recovery clears it without finishing a task',()=>{
 const fresh=new ReadFreshness(1000);fresh.failure();assert.equal(fresh.stale(1200),false);
 fresh.failure();fresh.failure();assert.equal(fresh.stale(2600),true);
 fresh.success(2700);assert.equal(fresh.stale(2800),false);assert.equal(fresh.stale(12700),true);
});

await test('one retained content host moves owners; stale cleanup cannot steal it',()=>{
 const original=globalThis.document;
 class Element{parentElement=null;className='';children=[];appendChild(node){if(node.parentElement)node.parentElement.children=node.parentElement.children.filter(child=>child!==node);this.children.push(node);node.parentElement=this;}}
 globalThis.document={createElement:()=>new Element()};
 try{
  const dock=new SessionContentDock(),parking=new Element(),ordinary=new Element(),workspace=new Element(),a={},b={};let scope={session:'a'};let changes=0;
  dock.subscribe(()=>changes++);dock.parkAt(parking);const same=dock.host;
  dock.present(a,ordinary,'a',()=>scope);assert.equal(dock.current().session,'a');
  dock.present(a,ordinary,'a',()=>scope);assert.equal(changes,1);
  dock.present(b,workspace,'b',()=>({session:'b'}));dock.release(a);
  assert.equal(dock.host,same);assert.equal(dock.host.parentElement,workspace);assert.equal(dock.current().session,'b');
  dock.release(b);assert.equal(dock.host.parentElement,parking);assert.equal(dock.current(),null);
 }finally{globalThis.document=original;}
});
