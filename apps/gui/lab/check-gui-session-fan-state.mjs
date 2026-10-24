import assert from 'node:assert/strict';
import {build} from 'esbuild';
const bundle=await build({stdin:{contents:"export * from './lab/gui-session-fan-state';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {thumbFanJoystick}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
let checks=0;const origin={x:350,y:500};
const make=(ids=['a','b','c'])=>{
 let now=0,next=0;const frames=new Map(),clock={now:()=>now,request:f=>{frames.set(++next,f);return next;},cancel:id=>frames.delete(id)};
 const fan=thumbFanJoystick(()=>{},clock);fan.start(origin,ids,'a');
 const advance=ms=>{for(let end=now+ms;now<end;){now=Math.min(end,now+16);const jobs=[...frames.values()];frames.clear();jobs.forEach(f=>f());}};
 const arc=(angle,radius=80)=>fan.move({x:origin.x-radius*Math.cos(angle*Math.PI/180),y:origin.y-radius*Math.sin(angle*Math.PI/180)});
 return {fan,advance,arc,frames};
};
for(const [gain,tier,interval] of [[14,1,1400],[32,2,1000],[56,3,700]]){
 const {fan,advance,arc}=make();advance(288);arc(45+gain/56*30);assert.equal(fan.read().hot,1);assert.equal(fan.read().tier,tier);assert.equal(fan.read().interval,interval);checks++;
 advance(interval-32);assert.equal(fan.read().hot,1);advance(48);assert.equal(fan.read().hot,2);checks++;
 arc(45);advance(4000);assert.equal(fan.read().hot,2);assert.equal(fan.read().direction,0);checks++;
 arc(37.5);assert.equal(fan.read().hot,1);assert.equal(fan.read().direction,-1);assert.deepEqual(fan.release(),{kind:'focus',id:'b'});assert.equal(fan.release().kind,'none');checks++;
}
for(const held of [false,true]){
 const {fan,advance,arc,frames}=make();if(held)advance(288);arc(45,145);assert.equal(fan.read().phase,'overview');advance(3000);assert.equal(fan.read().direction,0);assert.equal(fan.release().kind,'overview');assert.equal(frames.size,0);checks++;
 const retract=make();retract.advance(288);retract.arc(55);const preview=retract.fan.read().hot;retract.arc(45,145);retract.arc(45,130);assert(retract.fan.read().armed,'radial hysteresis');retract.arc(45,110);assert.equal(retract.fan.read().phase,'fan');assert.equal(retract.fan.read().hot,preview);retract.arc(35);assert.equal(retract.fan.read().hot,0);retract.fan.cancel();checks++;
}
{
 const {fan,advance,arc}=make();advance(288);arc(55);arc(45,145);assert.equal(fan.release().kind,'overview','overview remains accessible after arc selection');checks++;
}
{
 const {fan,advance,arc}=make();advance(288);arc(90,180);assert.equal(fan.read().armed,false,'straight up is not the overview gesture');assert.equal(fan.read().direction,0);arc(75,180);assert.equal(fan.read().armed,false,'ordinary angular steering does not open overview');fan.cancel();checks++;
}
{
 const {fan,advance,arc}=make();advance(288);arc(55,70);const target=fan.read().hot;for(const r of [85,100,120])arc(55,r);assert.equal(fan.read().hot,target);assert.equal(fan.read().tier,1,'radius does not steer the angle');fan.cancel();checks++;
}
for(const count of [0,1]){const {fan,advance,arc,frames}=make(['a'].slice(0,count));advance(288);arc(75);advance(5000);assert.equal(fan.release().kind,'none');assert.equal(frames.size,0);checks++;}
{
 const {fan,arc}=make();arc(60);assert.equal(fan.release().kind,'none');const tap=make();assert.equal(tap.fan.release().kind,'tap');assert.equal(tap.fan.release().kind,'none');checks++;
}
{
 const {fan,advance,arc,frames}=make();advance(288);arc(75);fan.cancel();advance(5000);assert.equal(fan.read().phase,'idle');assert.equal(frames.size,0);checks++;
}
console.log(`${checks} angular gears, radial overview/retraction, pause, reversal and cancellation checks passed.`);
