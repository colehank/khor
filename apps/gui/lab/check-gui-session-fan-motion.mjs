import assert from 'node:assert/strict';
import {build} from 'esbuild';
const bundle=await build({stdin:{contents:"export * from './lab/gui-session-fan-state';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {thumbFanJoystick}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
let checks=0;
function make(){let now=0,next=0;const jobs=new Map(),clock={now:()=>now,request:f=>{jobs.set(++next,f);return next;},cancel:id=>jobs.delete(id)},fan=thumbFanJoystick(()=>{},clock);const origin={x:350,y:500},neutral={x:325,y:475};fan.start(origin,['a','b','c','d','e'],'a');const advance=ms=>{for(let end=now+ms;now<end;){now=Math.min(end,now+16);const run=[...jobs.values()];jobs.clear();run.forEach(f=>f());}};advance(288);fan.move(neutral);return {fan,neutral,advance,move:(up,left=up*.35)=>fan.move({x:neutral.x-left,y:neutral.y-up})};}
// The old polar-only controller ignores this whole diagonal stroke.
for(const slope of [-.5,0,.35,1]){const {fan,move,advance}=make();move(18,18*slope);assert.equal(fan.read().hot,1,'upward straight/diagonal travel selects immediately');move(0,0);advance(4000);assert.equal(fan.read().hot,1);move(-28,28*Math.abs(slope));assert.equal(fan.read().hot,0,'the same diagonal path reverses');fan.cancel();checks++;}
{
 const {fan,move}=make();move(3,1);assert.equal(fan.read().hot,0);assert(fan.read().cursor>0,'cursor follows below selection threshold');const first=fan.read().cursor;move(6,2);assert(fan.read().cursor>first);assert.equal(fan.read().hot,0);fan.cancel();checks++;
}
{
 const {fan,move,advance}=make();for(const [up,left] of [[2,8],[-3,-9],[4,-5],[-4,5],[0,0]])move(up,left);advance(4000);assert.equal(fan.read().hot,0,'small hand tremor stays neutral');move(18);move(55);assert.equal(fan.read().hot,1,'continued travel changes speed without skipping candidates');move(0,0);const paused=fan.read().hot;advance(4000);assert.equal(fan.read().hot,paused);fan.cancel();checks++;
}
console.log(`${checks} diagonal travel, immediate subthreshold cursor, tremor, no skips and return-pause checks passed.`);
