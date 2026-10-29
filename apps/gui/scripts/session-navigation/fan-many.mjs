import assert from 'node:assert/strict';
import {sameDeckContactLayout} from '../../src/lib/deck-contact.ts';
import {fanFixture,deckJoystick,memoryClock} from './fixture.mjs';
let checks=0;
for(const count of [2,3,4,5,8,12,32]){
 const ids=Array.from({length:count},(_,i)=>`window-${i}`),f=fanFixture({ids});f.acquire();f.move(60);f.move(60);assert.equal(f.fan.read().hot,count-1);f.advance(176);const nextStep=()=>{const before=f.fan.read().pulse;for(let elapsed=0;f.fan.read().pulse===before&&elapsed<2000;elapsed+=16)f.advance(16);assert.equal(f.fan.read().pulse,before+1);f.advance(176);};const reached=new Set([f.fan.read().hot]);
 for(let i=0;i<count*2;i++){nextStep();reached.add(f.fan.read().hot);assert.equal(f.fan.read().hot,(count-2-i%count+count)%count);checks++;}
 assert.equal(reached.size,count);f.move(0);const paused=f.fan.read().hot;f.advance(4000);assert.equal(f.fan.read().hot,paused);checks++;
 f.move(-60);f.move(-60);f.advance(176);for(let i=0;i<count;i++){assert.equal(f.fan.read().hot,(paused+1+i)%count);nextStep();checks++;}f.fan.cancel();assert.equal(f.jobs.size,0);checks++;
}
for(const up of [17,40,60]){
 const f=fanFixture(),p=memoryClock(),pill=deckJoystick(5,0,()=>{},()=>{},()=>{},p.clock);pill.start(0,0);f.acquire();p.advance(288);f.move(up);pill.move(up*1.07,0);f.move(up);pill.move(up*1.07,0);
 f.advance(5000);p.advance(5000);assert.equal(f.fan.read().pulse,pill.read().pulse);assert.equal(f.fan.read().interval,pill.read().interval);assert.equal(f.fan.read().target,pill.read().target);assert(Math.abs(f.fan.read().position-pill.read().position)<1e-8);f.fan.cancel();pill.dispose();checks++;
}
const before=[336.54,780,36,36,390,844,390,844,0,0],after=[336,780,36,36,390,844,390,844,0,0];assert(sameDeckContactLayout(before,after));assert(!sameDeckContactLayout(before,[340,780,...after.slice(2)]));checks++;
console.log(`${checks} complete 2–32 cycles, common motion/rhythm and stable contact checks passed.`);
