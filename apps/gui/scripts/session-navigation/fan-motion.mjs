import assert from 'node:assert/strict';
import {fanFixture,deckJoystick,memoryClock} from './fixture.mjs';
import {fanFrame,fanPoint} from '../../src/components/session-nav/geometry.ts';
const layout={left:0,top:0,width:390,height:650,touch:44,gap:12,originX:354,originY:680};
let checks=0;
for(const slope of [-.5,0,.35,1]){const f=fanFixture();f.acquire();f.move(18,18*slope);assert.equal(f.fan.read().hot,0);assert(f.fan.read().position<0,'straight/diagonal upward travel follows before selection');const frame=fanFrame(layout);assert(fanPoint(frame,-f.fan.read().position).y<fanPoint(frame,0).y,'the same candidate physically follows the finger upward');f.move(40,40*slope);assert.equal(f.fan.read().hot,4);f.move(0,0);f.advance(4000);assert.equal(f.fan.read().hot,4);f.move(-60,60*Math.abs(slope));assert.equal(f.fan.read().hot,0,'the same diagonal path reverses after a deliberate threshold');f.fan.cancel();checks++;}
{
 const f=fanFixture();f.acquire();f.move(3,1);assert.equal(f.fan.read().hot,0);assert(f.fan.read().cursor>0);const first=f.fan.read().cursor;f.move(6,2);assert(f.fan.read().cursor>first);assert(f.fan.read().position<0);assert.equal(f.fan.read().hot,0);f.fan.cancel();checks++;
}
{
 const f=fanFixture();f.acquire();for(const [up,left] of [[2,8],[-3,-9],[4,-5],[-4,5],[0,0]])f.move(up,left);f.advance(4000);assert.equal(f.fan.read().hot,0);f.move(40);f.move(60);assert.equal(f.fan.read().hot,4);f.move(0,0);const paused=f.fan.read().hot;f.advance(4000);assert.equal(f.fan.read().hot,paused);f.fan.cancel();checks++;
}
{
 const time=memoryClock(),pill=deckJoystick(5,0,()=>{},()=>{},()=>{},time.clock);pill.start(0,0);time.advance(288);pill.move(17,0);const frame=fanFrame({...layout,placement:'bottom'});assert(fanPoint(frame,-pill.read().position).x>fanPoint(frame,0).x,'the bottom pill candidate physically follows a rightward thumb');assert.equal(pill.read().preview,0);pill.cancel();assert.equal(time.jobs.size,0);checks++;
}
console.log(`${checks} diagonal/bottom physical direction, continuous follow, tremor and pause checks passed.`);
