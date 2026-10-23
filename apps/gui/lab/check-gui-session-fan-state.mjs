import assert from 'node:assert/strict';
import {leftThumbFan,recentCandidates} from './gui-session-fan-state.ts';
let checks=0;
const origin={x:310,y:500},at=(angle,r=72)=>({x:origin.x-r*Math.cos(angle*Math.PI/180),y:origin.y-r*Math.sin(angle*Math.PI/180)});
const make=(ids=['a','b','c'])=>{const f=leftThumbFan(44,()=>{});f.start(origin,ids);return f;};
assert.deepEqual(recentCandidates(['current','b','unknown','a','b','new-tab:1','c','d'],['current','a','b','c','d','new-tab:1'],'current'),['b','a','c']);checks++;
for(const count of [0,1,2,3])for(const angle of [5,33,61]){
  const ids=['a','b','c'].slice(0,count),f=make(ids);f.hold();f.move(at(angle));const result=f.release();
  if(count)assert(ids.includes(result.id));else assert.equal(result.kind,'none');checks++;
}
for(const [angle,id] of [[8,'a'],[33,'b'],[58,'c']]){
  for(const radius of [32,72,115]){const f=make();f.hold();f.move(at(angle,radius));assert.deepEqual(f.release(),{kind:'focus',id});checks++;}
}
{
  const ids=['a','b','c'],f=make(ids);f.hold();ids.reverse();f.move(at(8));assert.equal(f.release().id,'a','the contact owns a frozen candidate order');checks++;
}
{
  const f=make();f.hold();f.move(at(10));f.move(at(58));f.move(origin);assert.equal(f.release().kind,'none','return to avatar cancels');checks++;
}
{
  const f=make();f.hold();f.move(at(33));f.move({x:origin.x,y:origin.y-90});assert.equal(f.release().kind,'none','fan cannot become overview midway');checks++;
}
for(const held of [false,true]){
  const f=make();if(held)f.hold();f.move({x:origin.x,y:origin.y-65});assert.equal(f.release().kind,'overview');checks++;
  const retract=make();if(held)retract.hold();retract.move({x:origin.x,y:origin.y-65});retract.move(origin);assert.equal(retract.release().kind,'none');checks++;
}
{
  const f=make();f.move({x:origin.x,y:origin.y-65});f.hold();f.move(at(33));assert.equal(f.release().kind,'none','overview cannot become fan midway');checks++;
}
for(const p of [{x:origin.x+80,y:origin.y-10},{x:origin.x-80,y:origin.y+70}]){const f=make();f.hold();f.move(p);assert.equal(f.release().kind,'none');checks++;}
{
  const f=make();f.move(at(8));assert.equal(f.release().kind,'none','a quick left flick is not a hold or a tap');checks++;
}
{
  const f=make();assert.equal(f.release().kind,'tap');assert.equal(f.release().kind,'none','native touch and pointer end cannot consume twice');checks++;
}
{
  const f=make();f.hold();f.move(at(21));assert.equal(f.read().hot,0);f.move(at(23));assert.equal(f.read().hot,0,'boundary tremor keeps the previous candidate');f.move(at(27));assert.equal(f.release().id,'b');checks++;
}
{
  const f=make();f.hold();f.move(at(8));f.cancel();assert.equal(f.release().kind,'none');checks++;
}
console.log(`${checks} recent-session ranking, leftward direct selection, direction locking, neutral cancellation and release checks passed.`);
