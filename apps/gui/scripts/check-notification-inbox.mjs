import assert from 'node:assert/strict';
import { collectNotifications, emptyInbox, restoreInbox, pendingNotification, sourceKey, INBOX_LIMIT } from '../src/lib/notification-inbox.ts';

const event = (id, at_ms, kind='turn_completed', resolved=false) => ({id,at_ms,kind,resolved,text:null,duration_ms:null,exit:null});
const row = (activity=[],extra={}) => ({id:'gui/a',execution_home:'a'.repeat(64),title:'A',imported:false,observation_owner:null,activity,...extra});
const options = {commandSeconds:30,onScreen:()=>false};
const collect = (inbox, rows, extra={}) => collectNotifications(inbox, rows, {...options,...extra});

let result = collect(emptyInbox(), [row([event('old',1),event('ask',2,'needs_input')])]);
assert.equal(result.items[0].read,true,'first snapshot does not present historical completion as new');
assert.equal(result.items[1].read,false,'outstanding requests survive first snapshot');
assert.equal(result.items.filter(pendingNotification).length,1);
assert.deepEqual(collect(result,[row([event('old',1),event('ask',2,'needs_input')])]),result,'repeated snapshots are idempotent');

let inbox = restoreInbox(JSON.stringify(result));
inbox.items[1].read=true;
assert(pendingNotification(inbox.items[1]),'reading is not approving');
inbox=collect(inbox,[row([event('ask',2,'needs_input',true),event('new',3)])]);
assert(!pendingNotification(inbox.items.find(i=>i.id==='ask')),'host resolution arrives from another device');
inbox=collect(inbox,[row([event('ask',2,'needs_input',false)])]);
assert(!pendingNotification(inbox.items.find(i=>i.id==='ask')),'older snapshots cannot undo resolution');
assert.equal(inbox.items.find(i=>i.id==='new').read,false);
assert.equal(collect(inbox,[row([event('new',3)])],{onScreen:()=>true}).items.find(i=>i.id==='new').read,true,'visible context counts as read');

const before=collect(emptyInbox(),[row()]);
result=collect(before,[row([event('catchup',500)])]);
assert.equal(result.items.length,1,'reconnect catches missed completion without observing the start');
const removed={...result,items:[]};
assert.equal(collect(removed,[row([event('catchup',500)])]).items.length,0,'removal retains deduplication cursor');

assert.equal(collect(emptyInbox(),[row([event('imported',3)],{imported:true})]).items.length,0,'imported histories are not implicitly subscribed');
const imported=row([event('followed',3)],{imported:true});
assert.equal(collect({...emptyInbox(),preferences:{[sourceKey(imported)]:true}},[imported]).items.length,1);
assert.equal(collect({...before,preferences:{[sourceKey(row())]:false}},[row([event('muted',3)])]).items[0].read,true,'mute retains owned history without unread reminders');
const sameIdOtherHome=row([event('same',10)],{execution_home:'b'.repeat(64)});
assert.equal(collect(emptyInbox(),[row([event('same',10)]),sameIdOtherHome]).items.length,2,'targets include the execution machine');
const command={...event('cmd',10,'command_finished'),duration_ms:31_000,exit:0};
assert.equal(collect(emptyInbox(),[row([command])]).items.length,1);
assert.equal(collect(emptyInbox(),[row([command])],{commandSeconds:60}).items.length,0);
assert.equal(collect(emptyInbox(),[row([command])],{commandSeconds:0}).items.length,0);
assert.equal(collect(emptyInbox(),[row([{...command,exit:130}])]).items.length,0,'cancellation is not failure');
const large=Array.from({length:1000},(_,i)=>event(String(i),i+1));
assert.equal(collect(emptyInbox(),[row(large)]).items.length,INBOX_LIMIT);
const pendingOnly=collect(emptyInbox(),[row([event('long-wait',1,'needs_input')])]);
const afterGap=collect(pendingOnly,[row(large.map(e=>({...e,at_ms:e.at_ms+10})))]);
assert.equal(afterGap.items.filter(pendingNotification).length,0,'later host turn events resolve requests older than the retained event tail');
const shellProgress=large.map(e=>({...e,kind:'command_finished',duration_ms:31_000}));
assert.equal(collect(pendingOnly,[row(shellProgress)]).items.filter(pendingNotification).length,1,'unrelated command progress neither resolves nor evicts a pending request');
assert.deepEqual(restoreInbox('{broken'),emptyInbox());
assert.equal(restoreInbox(JSON.stringify({...emptyInbox(),items:[{}]})).items.length,0);
assert.equal(collect(emptyInbox(),[row([event('invalid',NaN)])]).items.length,0);
console.log('notification inbox: replay, resolution, deduplication, mute, explicit follow, routing and bounds passed');
