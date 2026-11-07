import assert from 'node:assert/strict';
import {build} from 'esbuild';
const {outputFiles}=await build({stdin:{contents:"export * from './src/components/session-nav/action-motion';export * from './src/components/session-nav/geometry';export * from './src/lib/deck-intent';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false});
const {fanActionMotion,fanFrame,deckIntent,DECK_ACTION_REACH}=await import('data:text/javascript;base64,'+Buffer.from(outputFiles[0].text).toString('base64'));
let checks=0;
for(const placement of ['bottom','right']){
 const frame=fanFrame({left:0,top:0,width:390,height:728,touch:44,gap:12,originX:350,originY:752,placement});
 const reach=DECK_ACTION_REACH[placement],origin={x:350,y:752};
 const state=distance=>({origin,point:{x:origin.x-(placement==='right'?distance*Math.SQRT1_2:0),y:origin.y-distance*(placement==='right'?Math.SQRT1_2:1)},cancelArmed:false});
 assert(reach.overview-reach.create>=2.5,'A deliberate second reach separates New from All');
 let prior=-1;
 for(let i=0;i<=30;i++){
  const contact=state(frame.touch*(reach.create+(reach.overview+.6-reach.create)*i/30));
  const motion=fanActionMotion(frame,contact);assert(motion.transfer>=prior,'The exchange follows outward travel without snapping backward');prior=motion.transfer;
  assert(Math.abs(motion.newTab.grow+motion.overview.grow-1)<1e-9);
  assert.deepEqual(motion,fanActionMotion(frame,contact),'Returning to the same contact exactly retraces the shape');
  for(const cell of [motion.newTab,motion.overview])assert(cell.x-cell.width/2>=0&&cell.x+cell.width/2<=390&&cell.y-cell.height/2>=0&&cell.y+cell.height/2<=728);
  assert(!motion.bridge.includes('NaN'));
  checks++;
 }
 const near=fanActionMotion(frame,state(frame.touch*(reach.create+.3))),far=fanActionMotion(frame,state(frame.touch*(reach.overview+.6))),mid=fanActionMotion(frame,state(frame.touch*(reach.overview-.25)));
 assert.equal(near.newTab.grow,1);assert.equal(near.overview.grow,0);assert.equal(far.newTab.grow,0);assert.equal(far.overview.grow,1);
 assert(mid.opacity>.1&&mid.bridge&&mid.newTab.grow>.4&&mid.overview.grow>.4,'Independent lenses briefly attach while exchanging size');
 assert.equal(near.opacity,0);assert.equal(far.opacity,0);assert.equal(fanActionMotion(frame,{...state(500),cancelArmed:true}).newTab.grow,0);
 const before=state((reach.overview-.4)*44);assert.equal(deckIntent({dx:origin.x-before.point.x,up:origin.y-before.point.y,touch:44,placement,previous:'new'}),'new','Previewing the next lens cannot execute All before its threshold');
 assert.equal(fanActionMotion(frame,state(500),false).opacity,0);checks++;
}
console.log(`${checks} action separation, continuous reversible lens exchange, temporary adhesion and viewport bounds checks passed.`);
