import assert from 'node:assert/strict';import {build} from 'esbuild';import {sameDeckContactLayout} from '../src/lib/deck-contact.ts';
const bundle=await build({stdin:{contents:"export * from './lab/gui-session-fan-state';export * from './src/lib/deck-joystick';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {thumbFanJoystick,deckJoystick}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));let checks=0;
function time(){let now=0,next=0;const jobs=new Map();return {clock:{now:()=>now,request:f=>{jobs.set(++next,f);return next;},cancel:id=>jobs.delete(id)},advance(ms){for(let end=now+ms;now<end;){now=Math.min(end,now+16);const run=[...jobs.values()];jobs.clear();run.forEach(f=>f());}},jobs};}
const p=(a)=>({x:350-35*Math.cos(a*Math.PI/180),y:500-35*Math.sin(a*Math.PI/180)});
for(const count of [2,3,4,5,8,12,32]){
 const ids=Array.from({length:count},(_,i)=>`window-${i}`),t=time(),fan=thumbFanJoystick(()=>{},t.clock),first=1;fan.start({x:350,y:500},ids,ids[0]);t.advance(288);fan.move(p(45));fan.move(p(85));assert.equal(fan.read().hot,first);const reached=new Set([fan.read().hot]);
 for(let i=0;i<count*2;i++){t.advance(704);reached.add(fan.read().hot);assert.equal(fan.read().hot,(first+i+1)%count);checks++;}
 assert.equal(reached.size,count);fan.move(p(45));const paused=fan.read().hot;t.advance(4000);assert.equal(fan.read().hot,paused);checks++;
 fan.move(p(5));for(let i=0;i<count;i++){assert.equal(fan.read().hot,(paused-first-i+count*3)%count);t.advance(704);checks++;}fan.cancel();assert.equal(t.jobs.size,0);checks++;
}
// Manual detents are immediate; stationary repeat retains the production clock.
for(const [angle,gain] of [[61,18.5],[69,33.5],[85,56]]){
 const a=time(),b=time(),fan=thumbFanJoystick(()=>{},a.clock),pill=deckJoystick(5,0,()=>{},()=>{},()=>{},b.clock);fan.start({x:350,y:500},['a','b','c','d','e'],'a');pill.start(0,0);a.advance(288);b.advance(288);fan.move(p(45));fan.move(p(angle));pill.move(gain,0);
 const initialFan=fan.read().pulse,initialPill=pill.read().pulse;a.advance(5000);b.advance(5000);assert.equal(fan.read().pulse-initialFan,pill.read().pulse-initialPill);assert.equal(fan.read().interval,pill.read().interval);fan.cancel();pill.dispose();checks++;
}
const before=[336.54,780,36,36,390,844,390,844,0,0],after=[336,780,36,36,390,844,390,844,0,0];assert(sameDeckContactLayout(before,after));assert(!sameDeckContactLayout(before,[340,780,...after.slice(2)]));checks++;
console.log(`${checks} short-throw cycles across 2–32 windows, shared repeat rhythm and contact checks passed.`);
