import assert from 'node:assert/strict';
import {fanFixture,origin} from './fixture.mjs';
function make(first=32,home=origin){const f=fanFixture({home});f.acquire();f.move(first);return f;}
let checks=0;
{
 const f=make();f.fan.move(origin);assert.equal(f.fan.read().cancelArmed,true);assert.equal(f.fan.read().direction,0);assert.equal(f.fan.read().hot,4);f.advance(4000);assert.equal(f.fan.read().hot,4);assert.equal(f.fan.release().kind,'none');assert.equal(f.jobs.size,0);checks++;
}
{
 const f=make();f.fan.move(origin);f.fan.move({x:333,y:500});assert.equal(f.fan.read().cancelArmed,true);f.move(0);assert.equal(f.fan.read().cancelArmed,false);assert.equal(f.fan.read().hot,4);assert.deepEqual(f.fan.release(),{kind:'focus',id:'e'},'returning from home to arc neutral confirms the retained candidate');checks++;
}
{
 const f=make();f.move(36);f.move(5);assert.equal(f.fan.read().direction,0);for(const up of [2,4,6,0,-3]){f.move(up);assert.equal(f.fan.read().direction,0);}f.advance(5000);assert.equal(f.fan.read().hot,4);f.move(24);assert.equal(f.fan.read().direction,-1);assert.equal(f.fan.read().hot,4,'leaving neutral follows first');f.move(45);assert.equal(f.fan.read().hot,3);f.move(0);f.move(-12);assert.equal(f.fan.read().hot,3);f.move(-45);assert.equal(f.fan.read().hot,4);f.fan.cancel();checks++;
}
{
 const f=make();f.move(36);const hot=f.fan.read().hot;f.move(-30);assert.equal(f.fan.read().direction,0);for(const up of [-30,-31,-32])f.move(up);f.advance(4000);assert.equal(f.fan.read().hot,hot);f.move(-38);assert.equal(f.fan.read().direction,1);assert.equal(f.fan.read().hot,hot);f.move(-65);assert.equal(f.fan.read().hot,(hot+1)%5);f.fan.cancel();checks++;
}
{
 const f=make();f.fan.move({x:247.5,y:397.5});assert.equal(f.fan.read().armed,true);f.fan.move(origin);assert.equal(f.fan.read().cancelArmed,true);assert.equal(f.fan.release().kind,'none');checks++;
}
{
 const f=make(12);f.move(12);assert.equal(f.fan.read().hot,0);assert.equal(f.fan.read().direction,-1);f.advance(5000);assert.equal(f.fan.read().hot,0,'stationary subthreshold follow is never timed selection');f.move(32);f.advance(176+1440);assert.equal(f.fan.read().hot,3);f.move(0);assert.equal(f.fan.read().direction,0);f.fan.cancel();checks++;
}
for(const x of [335,365]){const home={x,y:500},f=make(32,home);f.fan.move(home);assert.equal(f.fan.read().cancelArmed,true);assert.equal(f.fan.release().kind,'none');checks++;}
console.log(`${checks} home cancellation, neutral confirmation, return pause and deliberate reverse checks passed.`);
