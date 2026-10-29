import assert from 'node:assert/strict';
import { build } from 'esbuild';
import { readFileSync } from 'node:fs';
import ts from 'typescript';
import {installAttachmentLeases} from './fixtures/attachment-leases.mjs';
const built = await build({entryPoints:['src/lib/attachment-view.ts'],bundle:true,write:false,format:'cjs',platform:'node'});
const module = {exports:{}};
Function('module','exports',built.outputFiles[0].text)(module,module.exports);
const {attachmentView} = module.exports;
const deferred = () => {let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no});return {promise,resolve,reject}};
const flush = () => new Promise(resolve => setImmediate(resolve));
const tests=[];
const test=(name,run)=>tests.push([name,run]);
const terminalSource = () => readFileSync(process.argv[2] ?? 'src/views/TerminalPane.tsx','utf8');
test('browser lease fixture requires explicit ACK, scope, one claim and active authority',()=>{
  const previous=globalThis.window;globalThis.window={};
  try {
    installAttachmentLeases();const wire=window.__attachmentLeases;
    const lease=wire('attachment_begin',{id:'a',kind:'chat'}).value;
    assert.match(lease,/^[0-9a-f]{32}$/);
    assert.throws(()=>wire('chat_stop',{id:'a',lease}),/not open/);
    assert.throws(()=>wire('chat_open',{id:'b',lease}),/scope mismatch/);
    assert.throws(()=>wire('term_open',{id:'a',lease}),/scope mismatch/);
    wire('chat_open',{id:'a',lease});assert.throws(()=>wire('chat_open',{id:'a',lease}),/already claimed/);
    wire('chat_stop',{id:'a',lease});wire('chat_leave',{id:'a',lease});
    assert.throws(()=>wire('chat_stop',{id:'a',lease}),/closed/);
  } finally {if(previous===undefined)delete globalThis.window;else globalThis.window=previous;}
});
test('dispose before begin sends no request or socket open',async()=>{
  const calls=[];
  const v=attachmentView('s','chat',{begin:async()=>{calls.push('begin');return 'key'},leave:async()=>calls.push('leave')});
  await v.dispose();
  await assert.rejects(v.open(async()=>calls.push('socket')),/closed/);
  assert.deepEqual(calls,[]);
});
test('late Begin ACK is released once and never opens a socket',async()=>{
  const begin=deferred(),calls=[];
  const v=attachmentView('s','terminal',{begin:()=>begin.promise,leave:async(...args)=>calls.push(args)});
  const open=v.open(async()=>{calls.push('socket');return true});
  await flush();const done=v.dispose();begin.resolve('late');
  await assert.rejects(open,/closed/);assert.deepEqual(await done,{failures:0,last:null});
  await v.dispose();assert.deepEqual(calls,[['s','terminal','late']]);
});
test('claimed slow open is cancelled without awaiting its completion',async()=>{
  const connecting=deferred(),entered=deferred(),calls=[];
  const v=attachmentView('s','chat',{begin:async()=> 'key',leave:async()=>calls.push('leave')});
  const open=v.open(()=>{entered.resolve();return connecting.promise});
  await entered.promise;await v.dispose();assert.deepEqual(calls,['leave']);
  connecting.resolve(true);await open;
  await assert.rejects(v.run(async()=>calls.push('input')),/closed/);
  assert.deepEqual(calls,['leave']);
});
test('open is claimed once and every command keeps its exact key',async()=>{
  const calls=[];
  const v=attachmentView('s','chat',{begin:async()=>{calls.push('begin');return 'key'},leave:async()=>calls.push('leave')});
  const one=v.open(async key=>{calls.push(['open',key]);return true});
  const two=v.open(async()=>{throw new Error('second claim')});assert.equal(one,two);await one;
  assert.equal(await v.run(async key=>key),'key');await v.dispose();
  assert.deepEqual(calls,['begin',['open','key'],'leave']);
});
test('failed opening cancels its registered lease and preserves original failure',async()=>{
  const original=new Error('connect'),calls=[];
  const v=attachmentView('s','chat',{begin:async()=> 'key',leave:async()=>calls.push('leave')});
  await assert.rejects(v.open(async()=>{throw original}),error=>error===original);
  assert.deepEqual(calls,['leave']);
});
test('cleanup failure is bounded, awaited and cannot mask the open error',async()=>{
  const original=new Error('connect');
  const v=attachmentView('s','chat',{begin:async()=> 'key',leave:()=>{throw new Error('x'.repeat(2000))}});
  await assert.rejects(v.open(async()=>{throw original}),error=>error===original);
  const result=await v.dispose();assert.equal(result.failures,1);assert.equal(result.last.length,512);
});
test('old facade cannot send to a new view of the same session',async()=>{
  let sequence=0;const inputs=[],leaves=[];
  const transport={begin:async()=>`key-${++sequence}`,leave:async(id,kind,key)=>leaves.push(key)};
  const old=attachmentView('s','terminal',transport);await old.open(async()=>true);await old.dispose();
  const next=attachmentView('s','terminal',transport);await next.open(async()=>true);
  await assert.rejects(old.run(async key=>inputs.push(key)),/closed/);
  await next.run(async key=>inputs.push(key));assert.deepEqual(inputs,['key-2']);assert.deepEqual(leaves,['key-1']);
  await next.dispose();
});
test('Begin failure creates no phantom leave or input authority',async()=>{
  const calls=[];const failure=new Error('admission');
  const v=attachmentView('s','chat',{begin:async()=>{throw failure},leave:async()=>calls.push('leave')});
  await assert.rejects(v.open(async()=>true),error=>error===failure);
  await assert.rejects(v.run(async()=>calls.push('input')),error=>error===failure);
  assert.deepEqual(await v.dispose(),{failures:0,last:null});assert.deepEqual(calls,[]);
});
test('production terminal sender never redirects queued old-view bytes after reopen',async()=>{
  const source = terminalSource();
  const start=source.indexOf('    let outbox:');
  const end=source.indexOf('    // **A paste of more than one line',start);
  assert(start>=0 && end>start,'locate the real production send closure');
  const js=ts.transpileModule(source.slice(start,end),{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText;
  const gate=deferred(),entered=deferred(),sent=[];
  const a={key:async bytes=>{sent.push(['A',bytes]);entered.resolve();await gate.promise}};
  const b={key:async bytes=>sent.push(['B',bytes])};
  const sender=Function('attachment','stopped','CHUNK_CHARS','notesScrollHealth','pollNow',`${js};return {send,change(view){attachment=view}};`)(a,false,16,()=>null,{current:()=>{}});
  sender.send('first-A');await entered.promise;
  sender.send('queued-A');sender.change(b);gate.resolve();await flush();
  assert.deepEqual(sent,[['A','first-A']],'queued A is discarded when its view is replaced');
  sender.send('new-B');await flush();assert.deepEqual(sent,[['A','first-A'],['B','new-B']]);
});
test('production terminal retry opens its own view while an old Begin ACK is pending',async()=>{
  const source=terminalSource();
  const start=source.indexOf('      const previous = attachment;');
  const end=source.indexOf('      return opened',start);
  assert(start>=0 && end>start,'locate the real production retry ownership block');
  const js=ts.transpileModule(source.slice(start,end),{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText;
  const disposal=deferred(),calls=[];
  const old={dispose:()=>{calls.push('retire-A');return disposal.promise}};
  const next={open:async()=>{calls.push('open-B');return null}};
  const begin=Function('attachment','again','id','terminalAttachment','attachmentRef','term','termHistory',`${js};return opened;`);
  const opened=begin(old,true,'s',()=>next,{current:old},{cols:80,rows:24},()=>0);
  await flush();assert.deepEqual(calls,['retire-A','open-B']);
  await opened;disposal.resolve({failures:0,last:null});await flush();
});
for(const [name,run] of tests){await run();console.log(`ok ${name}`)}
console.log(JSON.stringify({passed:tests.length}));
