import assert from 'node:assert/strict';import {build} from 'esbuild';
const bundle=await build({stdin:{contents:"export * from './lab/gui-session-fan-state';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {thumbFanJoystick,fanStick}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
let checks=0;const origin={x:350,y:500},point=(angle,r=35)=>({x:origin.x-r*Math.cos(angle*Math.PI/180),y:origin.y-r*Math.sin(angle*Math.PI/180)});
function make(ids=['a','b','c','d','e']){let now=0,next=0;const frames=new Map(),clock={now:()=>now,request:f=>{frames.set(++next,f);return next;},cancel:id=>frames.delete(id)},fan=thumbFanJoystick(()=>{},clock);fan.start(origin,ids,'a');const advance=ms=>{for(let end=now+ms;now<end;){now=Math.min(end,now+16);const run=[...frames.values()];frames.clear();run.forEach(f=>f());}};const arc=(a,r=35)=>fan.move(point(a,r));const acquire=()=>{advance(288);arc(45);assert.equal(fan.read().hot,0);};return {fan,advance,arc,acquire,frames};}
for(const [delta,tier,interval,manual] of [[16,1,1400,1],[24,2,1000,1],[40,3,700,1]]){
 const {fan,advance,arc,acquire}=make();acquire();arc(45+delta);assert.equal(fan.read().hot,manual,'further arc travel responds without waiting for auto repeat');assert.equal(fan.read().tier,tier);assert.equal(fan.read().interval,interval);checks++;
 advance(interval-32);assert.equal(fan.read().hot,manual);advance(48);assert.equal(fan.read().hot,(manual+1)%5);checks++;
 arc(45);const paused=fan.read().hot;advance(4000);assert.equal(fan.read().hot,paused);assert.equal(fan.read().direction,0);checks++;
 arc(29);assert.equal(fan.read().hot,(paused+4)%5);fan.release();assert.equal(fan.release().kind,'none');checks++;
}
{
 const {fan,arc,acquire,advance}=make();acquire();for(const a of [48,41,51,39,45])arc(a);advance(3000);assert.equal(fan.read().hot,0,'broad neutral absorbs small tremor');arc(61);arc(49);for(const a of [48,42,46])arc(a);advance(3000);assert.equal(fan.read().hot,1,'returning to neutral retains selection');checks++;fan.cancel();
}
{
 const {fan,arc,advance}=make();advance(288);arc(30);assert.equal(fan.read().hot,0,'first comfortable landing acquires neutral without jumping');arc(46);assert.equal(fan.read().hot,1);arc(30);advance(3000);assert.equal(fan.read().hot,1);checks++;fan.cancel();
}
{
 const {fan,arc,acquire}=make();acquire();arc(61);const target=fan.read().hot;for(const r of [40,55,80,100])arc(61,r);assert.equal(fan.read().hot,target,'radial travel does not jump candidates');arc(45,145);assert(fan.read().armed);arc(45,130);assert(fan.read().armed);arc(45,110);assert(!fan.read().armed);assert.equal(fan.read().hot,target);arc(29);assert.equal(fan.read().hot,0);checks++;fan.cancel();
}
for(const held of [false,true]){const {fan,arc,acquire}=make();if(held)acquire();arc(45,145);assert.equal(fan.release().kind,'overview');checks++;}
for(const count of [0,1,2,3]){const {fan,acquire,arc,frames}=make(['a','b','c'].slice(0,count));acquire();arc(85);assert.equal(fan.read().hot,count>1?1:0);fan.cancel();assert.equal(frames.size,0);checks++;}
for(const placement of ['right','bottom']){const neutral=placement==='right'?45:90,stick=fanStick(origin,44,placement);assert.equal(stick.move(point(neutral)).gain,0);assert(stick.move(point(neutral+16)).gain>0);assert.equal(stick.move(point(neutral)).gain,0);assert(stick.move(point(neutral-16)).gain<0);checks++;}
{
 const {fan,acquire,arc,frames,advance}=make();acquire();arc(75);fan.cancel();advance(5000);assert.equal(fan.read().phase,'idle');assert.equal(frames.size,0);checks++;
 const tap=make();assert.equal(tap.fan.release().kind,'tap');assert.equal(tap.fan.release().kind,'none');checks++;
}
for(const angle of [5,20,45,70,90]){const stick=fanStick(origin);assert.equal(stick.move(point(angle)).gain,0,'first landing never jumps at different thumb angles');checks++;}
{
 const {fan,arc,acquire,advance}=make();acquire();arc(61);advance(1000);arc(65);advance(1000);assert.equal(fan.read().hot,1,'moving the thumb restarts the reading interval');advance(500);assert.equal(fan.read().hot,2);fan.cancel();checks++;
 const continuous=make();continuous.acquire();for(const [angle,target] of [[61,1],[73,2],[85,3]]){continuous.arc(angle);assert.equal(continuous.fan.read().hot,target);}continuous.fan.cancel();checks++;
}
console.log(`${checks} short-throw acquisition, immediate detents, steady gears, broad neutral, reverse and radial overview checks passed.`);
