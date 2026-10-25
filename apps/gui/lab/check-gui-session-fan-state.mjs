import assert from 'node:assert/strict';import {build} from 'esbuild';
const bundle=await build({stdin:{contents:"export * from './lab/gui-session-fan-state';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {thumbFanJoystick,fanStick}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
let checks=0;const origin={x:350,y:500},neutral={x:325,y:475},point=(angle,r=35)=>({x:origin.x-r*Math.cos(angle*Math.PI/180),y:origin.y-r*Math.sin(angle*Math.PI/180)});
function make(ids=['a','b','c','d','e']){let now=0,next=0;const frames=new Map(),clock={now:()=>now,request:f=>{frames.set(++next,f);return next;},cancel:id=>frames.delete(id)},fan=thumbFanJoystick(()=>{},clock);fan.start(origin,ids,'a');const advance=ms=>{for(let end=now+ms;now<end;){now=Math.min(end,now+16);const run=[...frames.values()];frames.clear();run.forEach(f=>f());}};const sweep=(up,left=up*.35)=>fan.move({x:neutral.x-left,y:neutral.y-up});const acquire=()=>{advance(288);sweep(0);assert.equal(fan.read().hot,0);};return {fan,advance,sweep,acquire,frames};}
for(const [up,tier,interval] of [[17,1,1400],[36,2,1000],[55,3,700]]){
 const {fan,advance,sweep,acquire}=make();acquire();sweep(up);assert.equal(fan.read().hot,1,'first candidate responds without waiting for repeat');assert.equal(fan.read().tier,tier);assert.equal(fan.read().interval,interval);checks++;
 advance(interval-32);assert.equal(fan.read().hot,1);advance(48);assert.equal(fan.read().hot,2);checks++;
 sweep(0);const paused=fan.read().hot;advance(4000);assert.equal(fan.read().hot,paused);assert.equal(fan.read().direction,0);checks++;
 sweep(-24);assert.equal(fan.read().hot,(paused+4)%5);fan.release();assert.equal(fan.release().kind,'none');checks++;
}
{
 const {fan,sweep,acquire,advance}=make();acquire();for(const up of [3,-4,6,-6,0])sweep(up);advance(3000);assert.equal(fan.read().hot,0,'neutral absorbs small tremor');sweep(17);sweep(3);for(const up of [2,-3,0])sweep(up);advance(3000);assert.equal(fan.read().hot,1,'returning to neutral retains selection');checks++;fan.cancel();
}
{
 const {fan,advance}=make();advance(288);const p=point(30);fan.move(p);assert.equal(fan.read().hot,0,'first comfortable landing acquires neutral without jumping');fan.move({x:p.x-6,y:p.y-17});assert.equal(fan.read().hot,1);fan.move(p);advance(3000);assert.equal(fan.read().hot,1);checks++;fan.cancel();
}
{
 const {fan,sweep,acquire,advance}=make();acquire();sweep(17);const target=fan.read().hot;fan.move(point(45,145));assert(fan.read().armed);fan.move(point(45,130));assert(fan.read().armed);fan.move(point(45,110));assert(!fan.read().armed);assert.equal(fan.read().hot,target);advance(3000);assert.equal(fan.read().hot,target,'retraction pauses at a fresh comfortable neutral');const p=point(45,110);fan.move({x:p.x+6,y:p.y+24});assert.equal(fan.read().hot,0);checks++;fan.cancel();
}
for(const held of [false,true]){const {fan,acquire}=make();if(held)acquire();fan.move(point(45,145));assert.equal(fan.release().kind,'overview');checks++;}
for(const count of [0,1,2,3]){const {fan,acquire,sweep,frames}=make(['a','b','c'].slice(0,count));acquire();sweep(50);assert.equal(fan.read().hot,count>1?1:0);fan.cancel();assert.equal(frames.size,0);checks++;}
{
 const stick=fanStick(origin);assert.equal(stick.move(neutral).gain,0);assert(stick.move({x:325,y:458}).gain>0);assert.equal(stick.move(neutral).gain,0);assert(stick.move({x:325,y:500}).gain<0);checks++;
 const {fan,acquire,sweep,frames,advance}=make();acquire();sweep(50);fan.cancel();advance(5000);assert.equal(fan.read().phase,'idle');assert.equal(frames.size,0);checks++;
 const tap=make();assert.equal(tap.fan.release().kind,'tap');assert.equal(tap.fan.release().kind,'none');checks++;
}
for(const angle of [5,20,45,70,90]){const stick=fanStick(origin);assert.equal(stick.move(point(angle)).gain,0,'first landing never jumps at different thumb angles');checks++;}
{
 const {fan,sweep,acquire,advance}=make();acquire();sweep(16);advance(1000);sweep(22);advance(1000);assert.equal(fan.read().hot,1,'moving the thumb restarts the reading interval');advance(500);assert.equal(fan.read().hot,2);fan.cancel();checks++;
 const continuous=make();continuous.acquire();for(const [up,target] of [[17,1],[36,1],[55,1]]){continuous.sweep(up);assert.equal(continuous.fan.read().hot,target);}continuous.fan.cancel();checks++;
}
console.log(`${checks} short-throw acquisition, immediate first preview, wide gears, broad neutral, reverse and overview checks passed.`);
