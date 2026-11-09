import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
const source=readFileSync(new URL('../src/lib/type-size.ts',import.meta.url),'utf8');
function load(storage=new Map()) {
  let bodySize=13,other;
  const events=new Map(),documentEvents=new Map(),values=new Map();
  const window={localStorage:{getItem:key=>storage.get(key)??null,setItem:(key,value)=>storage.set(key,value)},addEventListener:(key,f)=>{const fs=events.get(key)??new Set();fs.add(f);events.set(key,fs);},removeEventListener:(key,f)=>events.get(key)?.delete(f),dispatchEvent:event=>{for(const f of events.get(event.type)??[])f(event);}};
  const document={documentElement:{style:{setProperty:(key,value)=>values.set(key,value)}},body:{appendChild(){}},visibilityState:'visible',createElement:()=>({style:{},remove(){}}),addEventListener:(key,f)=>documentEvents.set(key,f)};
  const exports={};
  vm.runInNewContext(ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText,{exports,window,document,navigator:{platform:'MacIntel',maxTouchPoints:0},CSS:{supports:()=>true},getComputedStyle:()=>({fontSize:String(bodySize)}),CustomEvent:class{constructor(type,{detail}){this.type=type;this.detail=detail;}},require:name=>name==='react'?{useEffect(){},useState(){}}:{heardFrom:(key,f)=>other=f}});
  return {api:exports,storage,values,system(value){bodySize=value;},foreground(){documentEvents.get('visibilitychange')();},other(){other();}};
}
const e=load(),a=e.api;
assert.equal(a.typeSize().termFollowSystem,true);
assert.equal(a.terminalSize(),14);
e.system(26);assert.equal(a.terminalSize(),28,'System accessibility size is not capped by the custom range');
a.setTypeSize({...a.typeSize(),followSystem:false,uiStep:0});
assert.equal(e.values.get('--text-term'),'28px','Custom interface scale does not override system terminal size');
a.setTypeSize({...a.typeSize(),term:18,termFollowSystem:false});assert.equal(a.terminalSize(),18);
a.setTypeSize({...a.typeSize(),termFollowSystem:true});assert.equal(a.terminalSize(),28);assert.equal(a.typeSize().term,18,'System mode preserves the custom value');
a.setTypeSize({...a.typeSize(),termFollowSystem:false});assert.equal(a.terminalSize(),18);
assert.equal(load(e.storage).api.typeSize().term,18);assert.equal(load(e.storage).api.typeSize().termFollowSystem,false);
for(const [term,follow] of [[14,true],[20,false]]) {
 const old=load(new Map([['khor.type-size',JSON.stringify({followSystem:true,uiStep:3,term})]]));
 assert.equal(old.api.typeSize().termFollowSystem,follow,'Legacy custom points remain custom');
}
a.setTypeSize({...a.typeSize(),termFollowSystem:true});a.wearTypeSize();const sizes=[];a.onTermSize(size=>sizes.push(size));e.system(19.5);e.foreground();
assert.equal(e.values.get('--text-term'),'21px');assert.equal(sizes.at(-1),21,'Returning to the app updates the live renderer too');
e.storage.set('khor.type-size',JSON.stringify({...a.typeSize(),termFollowSystem:false,term:20}));e.other();assert.equal(sizes.at(-1),20);
console.log('type size: independent system/manual modes, accessibility scale, preserved custom value, legacy migration, reload and foreground/other-window renderer updates passed');
