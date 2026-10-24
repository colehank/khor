import assert from 'node:assert/strict';
import {build} from 'esbuild';
const bundle=await build({stdin:{contents:"export * from './lab/gui-session-fan-state';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {thumbFanJoystick}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
let checks=0;
const origin={x:350,y:500};
const make=(ids=['a','b','c'],active='a')=>{
 let now=0,next=0;const frames=new Map(),clock={now:()=>now,request:f=>{frames.set(++next,f);return next;},cancel:id=>frames.delete(id)};
 const fan=thumbFanJoystick(()=>{},clock);fan.start(origin,ids,active,24);
 const advance=ms=>{for(let end=now+ms;now<end;){now=Math.min(end,now+16);const jobs=[...frames.values()];frames.clear();jobs.forEach(f=>f());}};
 return {fan,advance,move:(dx,up=0)=>fan.move({x:origin.x+dx,y:origin.y-up}),frames};
};
for(const [dx,tier,interval] of [[-14,1,1400],[-32,2,1000],[-60,3,700]]){
 const {fan,advance,move}=make();advance(288);move(dx,20);assert.equal(fan.read().hot,1);assert.equal(fan.read().tier,tier);assert.equal(fan.read().interval,interval);checks++;
 advance(interval-32);assert.equal(fan.read().hot,1);advance(48);assert.equal(fan.read().hot,2);checks++;
 move(0);const paused=fan.read().hot;advance(4000);assert.equal(fan.read().hot,paused);assert.equal(fan.read().direction,0);checks++;
 move(14);assert.equal(fan.read().hot,1);assert.equal(fan.read().direction,-1);assert.deepEqual(fan.release(),{kind:'focus',id:'b'});assert.equal(fan.release().kind,'none');checks++;
}
{
 const ids=['a','b','c'],{fan,advance,move}=make(ids);ids.reverse();advance(288);move(-14);assert.equal(fan.release().id,'b','contact freezes the pill order');checks++;
}
for(const held of [false,true]){
 const {fan,advance,move}=make();if(held)advance(288);move(0,75);assert.equal(fan.read().phase,'overview');assert.equal(fan.release().kind,'overview');assert.equal(fan.release().kind,'none');checks++;
 const retract=make();if(held)retract.advance(288);retract.move(0,75);retract.move(0);assert.equal(retract.fan.release().kind,'none');checks++;
}
{
 const {fan,advance,move}=make();advance(288);move(-14);move(0,75);assert.equal(fan.read().lane,'fan');assert.equal(fan.read().direction,0);assert.deepEqual(fan.release(),{kind:'focus',id:'b'},'return to neutral pauses and retains preview');checks++;
}
{
 const {fan,advance,move}=make();move(0,75);advance(5000);move(-50,20);assert.equal(fan.read().lane,'overview');assert.equal(fan.release().kind,'none');checks++;
}
{
 const {fan,move}=make();move(-20);assert.equal(fan.release().kind,'none','quick flick is neither held nor tap');checks++;
 const tap=make();assert.equal(tap.fan.release().kind,'tap');assert.equal(tap.fan.release().kind,'none');checks++;
}
for(const count of [0,1]){
 const {fan,advance,move,frames}=make(['a'].slice(0,count));advance(288);move(-60);advance(5000);assert.equal(fan.release().kind,'none');assert.equal(frames.size,0);checks++;
}
{
 const {fan,advance,move,frames}=make();advance(288);move(-60);fan.cancel();advance(5000);assert.equal(fan.read().phase,'idle');assert.equal(fan.release().kind,'none');assert.equal(frames.size,0);checks++;
}
console.log(`${checks} production joystick gears, pause, reversal, order, lane ownership and cancellation checks passed.`);
