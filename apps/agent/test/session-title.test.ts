import {test} from 'node:test';
import assert from 'node:assert/strict';
import {generateSessionTitle} from '../src/session-title.ts';
import {openModels,pickModel} from '../src/models.ts';
import type {ModelRuntime} from '@earendil-works/pi-coding-agent';

const model=pickModel(openModels({provider:'newapi',key:'fixture',baseUrl:'http://127.0.0.1:9'}),'fixture').model;
function runtime(text:string,stopReason='stop') {
  const calls:unknown[]=[];
  return {calls,value:{completeSimple:async(...args:unknown[])=>{calls.push(args);return {content:[{type:'text',text}],stopReason};}} as unknown as Pick<ModelRuntime,'completeSimple'>};
}
test('title generation is bounded, separate from conversation messages, and has no tools',async()=>{
  const fake=runtime('“设备网络整理”');
  assert.equal(await generateSessionTitle(fake.value,model,'x'.repeat(6000),new AbortController().signal),'设备网络整理');
  const [,context,options]=fake.calls[0] as [unknown,{messages:{content:string}[];tools?:unknown},{signal:AbortSignal}];
  assert.equal(context.messages.length,1);
  assert.equal(context.messages[0]!.content.length,4000);
  assert.equal(context.tools,undefined);
  assert(options.signal instanceof AbortSignal);
});
test('failed or multiline naming cannot publish a bogus session title',async()=>{
  for(const [text,reason] of [['','stop'],['Title\nextra','stop'],['Error text','error'],['partial','aborted']]) {
    assert.equal(await generateSessionTitle(runtime(text!,reason).value,model,'prompt',new AbortController().signal),null);
  }
});
