import assert from 'node:assert/strict';
import {build} from 'esbuild';
import {sameDeckContactLayout} from '../src/lib/deck-contact.ts';
const bundle=await build({stdin:{contents:"export * from './lab/gui-session-fan-state';export * from './src/lib/deck-joystick';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {thumbFanJoystick,deckJoystick}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
let checks=0;
const point=(x,y,gain)=>({x:x-80*Math.cos((45+gain/56*30)*Math.PI/180),y:y-80*Math.sin((45+gain/56*30)*Math.PI/180)});
function time(){let now=0,next=0;const jobs=new Map();return {clock:{now:()=>now,request:f=>{jobs.set(++next,f);return next;},cancel:id=>jobs.delete(id)},advance(ms){for(let end=now+ms;now<end;){now=Math.min(end,now+16);const run=[...jobs.values()];jobs.clear();run.forEach(f=>f());}},jobs};}
for(const count of [2,3,4,5,8,12,32]){
 const ids=Array.from({length:count},(_,i)=>`window-${i}`),t=time(),fan=thumbFanJoystick(()=>{},t.clock);
 fan.start({x:350,y:500},ids,ids[0]);t.advance(288);fan.move(point(350,500,56));const reached=new Set([fan.read().hot]);
 for(let i=0;i<count*2;i++){t.advance(704);reached.add(fan.read().hot);assert.equal(fan.read().hot,(i+2)%count);checks++;}
 assert.equal(reached.size,count);fan.move({x:350,y:500});const paused=fan.read().hot;t.advance(4000);assert.equal(fan.read().hot,paused);checks++;
 fan.move(point(350,500,-56));for(let i=0;i<count;i++){assert.equal(fan.read().hot,(paused-1-i+count*2)%count);t.advance(704);checks++;}
 fan.cancel();assert.equal(t.jobs.size,0);checks++;
}
// The adapter must preserve the pill's exact rhythmic sequence, including
// gear changes, neutral pause and reverse. Only the angular input is adapted.
{
 const a=time(),b=time(),fan=thumbFanJoystick(()=>{},a.clock),pill=deckJoystick(5,0,()=>{},()=>{},()=>{},b.clock);
 fan.start({x:300,y:500},['a','b','c','d','e'],'a');pill.start(0,0);a.advance(288);b.advance(288);
 for(const [dx,ms] of [[14,1410],[32,2200],[56,3500],[0,3000],[-14,2800],[-56,1400],[0,500]]){
  fan.move(point(300,500,dx));pill.move(dx,0);a.advance(ms);b.advance(ms);
  const f=fan.read(),p=pill.read();assert.deepEqual([f.hot,f.direction,f.tier,f.interval,f.pulse],[p.preview,p.direction,p.tier,p.interval,p.pulse]);checks++;
 }
 fan.cancel();pill.dispose();
}
{
 const before=[336.54,780,36,36,390,844,390,844,0,0],after=[336,780,36,36,390,844,390,844,0,0];assert(sameDeckContactLayout(before,after));assert(!sameDeckContactLayout(before,[...after.slice(0,4),430,844,430,844,0,0]));assert(!sameDeckContactLayout(before,[340,780,...after.slice(2)]));checks++;
}
console.log(`${checks} bidirectional cycles across 2–32 windows, exact pill parity and contact checks passed.`);
