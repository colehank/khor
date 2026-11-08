import assert from 'node:assert/strict';
import {deckJoystick,memoryClock,fanFixture,pointAt} from './fixture.mjs';

let checks=0;
{
 const timing=memoryClock(),commits=[],actions=[];
 const engine=deckJoystick(5,0,()=>{},i=>commits.push(i),()=>actions.push('all'),timing.clock,undefined,()=>actions.push('new'));
 engine.start(0,0,44);timing.advance(288);engine.move(32,0);timing.advance(176);
 engine.move(0,-110);assert.equal(engine.read().intent,'new');
 const parked=engine.read();engine.move(20,-57.2);
 assert.notEqual(engine.read().intent,'switch','Returning New must not hand the point back while the entry glass is still deformed');
 timing.advance(5000);assert.equal(engine.read().position,parked.position,'Candidates stay frozen through the entire return handoff');
 assert.equal(engine.read().preview,parked.preview);assert.equal(engine.release(),'none','Releasing an in-flight point keeps the opened session');
 assert.deepEqual(commits,[]);assert.deepEqual(actions,[]);checks++;
}
{
 const f=fanFixture();f.acquire();f.move(32);f.advance(176);f.fan.move(pointAt(45,155));
 assert.equal(f.fan.read().intent,'new');const parked=f.fan.read();f.fan.move(pointAt(45,110));
 assert.notEqual(f.fan.read().intent,'switch','The GUI point also waits for the complete glass return');
 f.advance(5000);assert.equal(f.fan.read().position,parked.position);assert.equal(f.fan.read().hot,parked.hot);
 assert.equal(f.fan.read().cursor,parked.cursor,'The parked arc position is not reset during the return');
 assert.equal(f.fan.release().kind,'none');checks++;
}
console.log(`${checks} complete point handoff, frozen reading and in-flight release checks passed.`);
