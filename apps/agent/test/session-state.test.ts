import {test} from 'node:test';
import assert from 'node:assert/strict';
import {configOptions,usageUpdate,applySessionOptions} from '../src/session-state.ts';
import {openModels,pickModel} from '../src/models.ts';
import type {AgentSession} from '@earendil-works/pi-coding-agent';

test('thinking choices follow the actual SDK model and stale choices are normalized',()=>{
  const seated=openModels({provider:'anthropic',key:'fixture'});
  const model=pickModel(seated,'claude-sonnet-4-6').model;
  const options=configOptions(model,[model],{thought_level:'not-a-real-level'});
  const thinking=options.find(v=>v.id==='thought_level');
  assert(thinking&&'options' in thinking);
  assert(thinking.options!.some(v=>v.value===thinking.currentValue));
  assert(!configOptions(model,[model],{},false).some(v=>v.id==='thought_level'),'unlisted sibling metadata does not promise a thinking control');
  const plain={...model,reasoning:false};
  assert(!configOptions(plain,[plain],{thought_level:'high'}).some(v=>v.id==='thought_level'));
});
test('context is a measured snapshot and unknown/compacted usage is not fabricated',()=>{
  let tokens:number|null=4200;
  const session={getContextUsage:()=>({tokens,contextWindow:200000,percent:2.1}),getSessionStats:()=>({cost:0.12})} as Pick<AgentSession,'getContextUsage'|'getSessionStats'>;
  assert.equal(usageUpdate(session)?.used,4200);assert.equal(usageUpdate(session)?.cost?.amount,0.12);
  tokens=1000;assert.equal(usageUpdate(session)?.used,1000,'new measurements replace previous ones');
  tokens=null;assert.equal(usageUpdate(session),null,'Pi cannot measure immediately after compaction');
  tokens=1000;assert.equal(usageUpdate(session,false),null,'a guessed gateway window is never a gauge');
});
test('invalid values never reach native Pi setters',()=>{
  const changes:unknown[]=[];
  const fake={getAvailableThinkingLevels:()=>['off','low'],setThinkingLevel:(v:unknown)=>changes.push(v),setAutoCompactionEnabled:(v:unknown)=>changes.push(v)} as unknown as AgentSession;
  assert.throws(()=>applySessionOptions(fake,{thought_level:'max'}));assert.deepEqual(changes,[]);
  applySessionOptions(fake,{thought_level:'low',auto_compaction:false});assert.deepEqual(changes,['low',false]);
});
