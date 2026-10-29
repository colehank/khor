import assert from 'node:assert/strict';
import {fanFixture,pointAt} from './fixture.mjs';
let checks=0;
for(const [up,tier,interval] of [[32,1,1400],[40,2,1000],[60,3,700]]){
 const f=fanFixture();f.acquire();f.move(17);assert.equal(f.fan.read().hot,0);assert(f.fan.read().position<0,'preselection follows before the threshold');f.move(32);f.move(up);assert.equal(f.fan.read().hot,4);assert.equal(f.fan.read().tier,tier);assert.equal(f.fan.read().interval,interval);checks++;
 f.advance(176);f.advance(interval-32);assert.equal(f.fan.read().hot,4);f.advance(48);assert.equal(f.fan.read().hot,3);checks++;
 f.move(0);const paused=f.fan.read().hot;f.advance(4000);assert.equal(f.fan.read().hot,paused);assert.equal(f.fan.read().direction,0);checks++;
 f.move(-50);assert.equal(f.fan.read().hot,(paused+1)%5);f.fan.release();assert.equal(f.fan.release().kind,'none');assert.equal(f.jobs.size,0);checks++;
}
{
 const f=fanFixture();f.acquire();for(const up of [3,-4,6,-6,0])f.move(up);f.advance(3000);assert.equal(f.fan.read().hot,0,'small hand tremor never crosses the selection threshold');f.move(32);f.move(0);for(const up of [2,-3,0])f.move(up);f.advance(3000);assert.equal(f.fan.read().hot,4,'returning to neutral retains the accepted target');checks++;f.fan.cancel();
}
{
 const f=fanFixture();f.advance(288);const point=pointAt(30);f.fan.move(point);assert.equal(f.fan.read().hot,0);f.fan.move({x:point.x-6,y:point.y-17});assert.equal(f.fan.read().hot,0);assert(f.fan.read().position<0);f.fan.move({x:point.x-12,y:point.y-40});assert.equal(f.fan.read().hot,4);f.fan.move(point);f.advance(3000);assert.equal(f.fan.read().hot,4);checks++;f.fan.cancel();
}
{
 const f=fanFixture();f.acquire();f.move(32);const target=f.fan.read().hot;f.fan.move(pointAt(45,145));assert(f.fan.read().armed);f.fan.move(pointAt(45,130));assert(f.fan.read().armed);f.fan.move(pointAt(45,110));assert(!f.fan.read().armed);assert.equal(f.fan.read().hot,target);f.advance(3000);assert.equal(f.fan.read().hot,target,'overview retraction parks at a fresh neutral');const point=pointAt(45,110);f.fan.move({x:point.x+12,y:point.y+50});assert.equal(f.fan.read().hot,0);checks++;f.fan.cancel();
}
for(const held of [false,true]){const f=fanFixture();if(held)f.acquire();f.fan.move(pointAt(45,145));assert.equal(f.fan.release().kind,held?'overview':'none','the right fan requires the long-press before overview; a quick stray stroke is inert');checks++;}
for(const count of [0,1,2,3]){const f=fanFixture({ids:['a','b','c'].slice(0,count)});f.acquire();f.move(50);assert.equal(f.fan.read().hot,count>1?count-1:0);f.fan.cancel();assert.equal(f.jobs.size,0);checks++;}
{
 const f=fanFixture();f.acquire();f.move(17);assert.equal(f.fan.read().hot,0);assert(f.fan.read().position<0);f.move(0);f.advance(3000);assert.equal(f.fan.read().position,0);checks++;f.fan.cancel();
 const canceled=fanFixture();canceled.acquire();canceled.move(50);canceled.fan.cancel();canceled.advance(5000);assert.equal(canceled.fan.read().phase,'idle');assert.equal(canceled.jobs.size,0);checks++;
 const tap=fanFixture();assert.equal(tap.fan.release().kind,'tap');assert.equal(tap.fan.release().kind,'none');checks++;
}
for(const angle of [5,20,45,70,90]){const f=fanFixture();f.advance(288);f.fan.move(pointAt(angle));assert.equal(f.fan.read().hot,0,'first comfortable landing never jumps at different thumb angles');f.fan.cancel();checks++;}
{
 const f=fanFixture();f.acquire();f.move(16);f.advance(1000);f.move(22);f.advance(3000);assert.equal(f.fan.read().hot,0,'stationary subthreshold offset never cycles');f.move(32);f.advance(176+1200);assert.equal(f.fan.read().hot,4);f.advance(240);assert.equal(f.fan.read().hot,3);f.fan.cancel();checks++;
 const f2=fanFixture();f2.acquire();for(const up of [32,40,60]){f2.move(up);assert.equal(f2.fan.read().hot,4,'speed changes never skip another candidate');}f2.fan.cancel();checks++;
}
console.log(`${checks} follow threshold, acquisition, collection, overview and lifecycle checks passed.`);
