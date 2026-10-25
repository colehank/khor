import assert from 'node:assert/strict';import {build} from 'esbuild';
const bundle=await build({stdin:{contents:"export * from './lab/gui-session-fan-state';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {thumbFanJoystick}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
const origin={x:350,y:500};
function make(first=17,home=origin){let now=0,next=0;const jobs=new Map(),clock={now:()=>now,request:f=>{jobs.set(++next,f);return next;},cancel:id=>jobs.delete(id)},fan=thumbFanJoystick(()=>{},clock);fan.start(origin,['a','b','c','d','e'],'a',44,home);const advance=ms=>{for(let end=now+ms;now<end;){now=Math.min(end,now+16);const run=[...jobs.values()];jobs.clear();run.forEach(f=>f());}};const move=up=>fan.move({x:325-up*.35,y:475-up});advance(288);move(0);move(first);return {fan,move,advance,jobs};}
let checks=0;
{
 const {fan,advance,jobs}=make();fan.move(origin);assert.equal(fan.read().cancelArmed,true,'return to original avatar arms cancellation');assert.equal(fan.read().direction,0);assert.equal(fan.read().hot,1);advance(4000);assert.equal(fan.read().hot,1);assert.equal(fan.release().kind,'none');assert.equal(jobs.size,0);checks++;
}
{
 const {fan,move}=make();fan.move(origin);fan.move({x:333,y:500});assert.equal(fan.read().cancelArmed,true,'small edge tremor does not disarm cancel');move(0);assert.equal(fan.read().cancelArmed,false);assert.equal(fan.read().hot,1);assert.deepEqual(fan.release(),{kind:'focus',id:'b'},'arc neutral still confirms the retained preview');checks++;
}
{
 const {fan,move,advance}=make();move(36);move(10);assert.equal(fan.read().direction,0,'returning toward neutral brakes early');for(const up of [12,14,16,18,12]){move(up);assert.equal(fan.read().direction,0);}advance(5000);assert.equal(fan.read().hot,1,'return tremor does not restart cycling');move(24);assert.equal(fan.read().direction,1);move(0);move(-17);assert.equal(fan.read().direction,0,'small neutral overshoot does not reverse');move(-24);assert.equal(fan.read().direction,-1);fan.cancel();checks++;
}
{
 const {fan,move,advance}=make();move(36);const hot=fan.read().hot;move(-30);assert.equal(fan.read().direction,0,'coalesced jump across neutral brakes before reversing');for(const up of [-30,-31,-32])move(up);advance(4000);assert.equal(fan.read().hot,hot);move(-38);assert.equal(fan.read().direction,-1,'deliberate continued movement reverses');fan.cancel();checks++;
}
{
 const {fan}=make();fan.move({x:247.5,y:397.5});assert.equal(fan.read().armed,true);fan.move(origin);assert.equal(fan.read().cancelArmed,true);assert.equal(fan.release().kind,'none','home cancellation also retracts overview');checks++;
}
{
 const {fan,move,advance}=make(12);move(12);assert.equal(fan.read().direction,1,'duplicate stationary events do not brake a light initial push');advance(1440);assert.equal(fan.read().hot,2);move(10);assert.equal(fan.read().direction,0,'a real return stroke still brakes');fan.cancel();checks++;
}
for(const x of [335,365]){
 const home={x,y:500},{fan}=make(17,home);fan.move(home);assert.equal(fan.read().cancelArmed,true,'an off-centre press can return to the actual avatar centre');assert.equal(fan.release().kind,'none');checks++;
}
console.log(`${checks} home cancellation, cancel tremor, neutral confirmation, return braking and deliberate reverse checks passed.`);
