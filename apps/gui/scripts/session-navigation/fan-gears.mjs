import assert from 'node:assert/strict';
import {fanFixture} from './fixture.mjs';
let checks=0;
for(const sign of [-1,1]){
 const f=fanFixture();f.acquire();f.move(sign*17);assert.equal(f.fan.read().pulse,0,'small movement follows without selecting');f.move(sign*32);assert.equal(f.fan.read().pulse,1);f.move(sign*30);assert.equal(f.fan.read().tier,1);assert.equal(f.fan.read().pulse,1);checks++;
 f.move(sign*36);assert.equal(f.fan.read().tier,2);assert.equal(f.fan.read().interval,1000);for(const up of [31,34,30,33,35]){f.move(sign*up);assert.equal(f.fan.read().tier,2);assert.equal(f.fan.read().pulse,1);}checks++;
 f.advance(720);f.move(sign*24);assert.equal(f.fan.read().tier,1);f.advance(1200);assert.equal(f.fan.read().pulse,1,'downshift provides a full reading interval');f.advance(240);assert.equal(f.fan.read().pulse,2);checks++;
 f.move(sign*55);assert.equal(f.fan.read().tier,3);assert.equal(f.fan.read().pulse,2,'gear changes never skip candidates');for(const up of [48,45,50]){f.move(sign*up);assert.equal(f.fan.read().tier,3);}f.move(sign*40);assert.equal(f.fan.read().tier,2);checks++;
 f.move(0);const paused=f.fan.read().pulse;f.advance(5000);assert.equal(f.fan.read().pulse,paused);f.move(-sign*24);assert.equal(f.fan.read().pulse,paused);assert.notEqual(f.fan.read().position,f.fan.read().target,'reverse follows continuously before snapping');f.move(-sign*45);assert.equal(f.fan.read().pulse,paused+1);assert.equal(f.fan.read().tier,2,'deliberate reverse keeps the gear shown during continuous preselection');assert.equal(f.fan.read().interval,1000);f.fan.cancel();assert.equal(f.jobs.size,0);checks++;
}
console.log(`${checks} wide gears, directional tremor, full reading interval and continuous reverse checks passed.`);
