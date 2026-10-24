import assert from 'node:assert/strict';
import {sameDeckContactLayout} from '../src/lib/deck-contact.ts';
import {leftThumbFan,recentCandidates} from './gui-session-fan-state.ts';
let checks=0;
for(const count of [2,3,4,5,8,12,32]){
  const ids=Array.from({length:count},(_,i)=>`window-${i}`);
  assert.equal(recentCandidates(ids.slice(0,2),['active',...ids],'active').length,count,'all open windows must be available, including unvisited ones');checks++;
  const gesture=leftThumbFan(44,()=>{}),origin={x:320,y:500};gesture.start(origin,ids);gesture.hold();
  for(let i=0;i<count;i++){
    const angle=i/(count-1)*66*Math.PI/180;
    gesture.move({x:origin.x-80*Math.cos(angle),y:origin.y-80*Math.sin(angle)});
    assert.equal(gesture.read().ids[gesture.read().hot],ids[i],`window ${i+1}/${count} must be reachable`);checks++;
  }
  assert.deepEqual(gesture.release(),{kind:'focus',id:ids.at(-1)});assert.equal(gesture.release().kind,'none');checks++;
}
{
 const before=[336.54,780,36,36,390,844,390,844,0,0],after=[336,780,36,36,390,844,390,844,0,0];assert(sameDeckContactLayout(before,after));assert(!sameDeckContactLayout(before,[...after.slice(0,4),430,844,430,844,0,0]));assert(!sameDeckContactLayout(before,[340,780,...after.slice(2)]));checks++;
}
console.log(`${checks} all-window fan reachability and release checks passed.`);
