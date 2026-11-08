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

for(const placement of ['bottom','right'])for(const destination of ['new','all']){
 const timing=memoryClock(),events=[];
 const root=placement==='bottom'?deckJoystick(5,0,()=>{},()=>events.push('focus'),()=>events.push('all'),timing.clock,undefined,()=>events.push('new')):null;
 const gui=placement==='right'?fanFixture():null;
 if(root){root.start(0,0,44);timing.advance(288);root.move(32,0);timing.advance(176);}else{gui.acquire();gui.move(32);gui.advance(176);}
 const read=()=>root?root.read():gui.fan.read();
 const move=distance=>root?root.move(0,-distance):gui.fan.move(pointAt(45,distance));
 const advance=ms=>root?timing.advance(ms):gui.advance(ms);
 move(destination==='new'?(root?110:155):280);
 const saved=read(),start=placement==='bottom'?.6:2;
 const far=placement==='bottom'?4.8:5.8;
 for(const forward of destination==='all'?[far-.2,far-.6,far-1.1]:[start+.9,start+.6,start+.3,start+.02]){
  move(forward*44);assert.equal(read().intent,'handoff');
  advance(3200);assert.equal(read().position,saved.position);assert.equal(read().target,saved.target);assert.equal(read().cursor,saved.cursor);
 }
 if(destination==='new'){
  move((start-.02)*44);assert.equal(read().intent,'switch');
  assert.equal(read().position,saved.position,'Landing cannot replay motion accumulated while suspended');
  assert.equal(read().cursor,saved.cursor,'Landing retains the departure point instead of centering it');
  advance(3200);assert.equal(read().position,saved.position);
  if(root)root.move(16,-(start-.02)*44);else{const p=pointAt(45,(start-.02)*44);gui.fan.move({x:p.x+6,y:p.y+12});}
  assert.notEqual(read().cursor,saved.cursor,'A deliberate fresh arc movement resumes control');
  if(root)root.cancel();else gui.fan.cancel();
 }else{
  if(root)assert.equal(root.release(),'none');else assert.equal(gui.fan.release().kind,'none');
  assert.deepEqual(events,[]);
 }
 checks++;
}
{
 const coarse=fanFixture(),fine=fanFixture();
 for(const f of [coarse,fine]){f.acquire();f.move(18,18);}
 coarse.move(40,40);for(let up=19;up<=40;up++)fine.move(up,up);
 assert.equal(coarse.fan.read().hot,4,'A coalesced diagonal event preserves the selection before leaving the arc');
 assert.equal(coarse.fan.read().hot,fine.fan.read().hot);assert.equal(coarse.fan.read().pulse,fine.fan.read().pulse);
 assert.equal(coarse.fan.read().intent,'handoff');coarse.fan.cancel();fine.fan.cancel();checks++;
}
console.log(`${checks} bottom/GUI entry and exchange stops, full rejoin, rebase, coalesced boundaries and safe releases passed.`);
