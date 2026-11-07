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
  assert(!/NaN|Infinity/.test(motion.shape.path));
  checks++;
 }
 const near=fanActionMotion(frame,state(frame.touch*(reach.create+.3))),far=fanActionMotion(frame,state(frame.touch*(reach.overview+.6))),mid=fanActionMotion(frame,state(frame.touch*(reach.overview-.25)));
 assert.equal(near.newTab.grow,1);assert.equal(near.overview.grow,0);assert.equal(far.newTab.grow,0);assert.equal(far.overview.grow,1);
 assert(mid.shape.joined&&mid.shape.neck>frame.touch*.1&&mid.newTab.grow>.4&&mid.overview.grow>.4,'The glass silhouette itself grows a waist while exchanging size');
 assert.equal(mid.shape.path.match(/M /g).length,1,'Connected glass has one outer contour without overlapping capsule rims');
 for(const endpoint of [near,far]){assert.equal(endpoint.shape.joined,false);assert.equal(endpoint.shape.pull,0);assert.equal(endpoint.shape.path.match(/M /g).length,2);}
 assert.equal(fanActionMotion(frame,{...state(500),cancelArmed:true}).newTab.grow,0);
 const before=state((reach.overview-.4)*44);assert.equal(deckIntent({dx:origin.x-before.point.x,up:origin.y-before.point.y,touch:44,placement,previous:'new'}),'new','Previewing the next lens cannot execute All before its threshold');
 assert.equal(fanActionMotion(frame,state(500),false).shape.joined,false);checks++;
}
for(const height of [180,728])for(const placement of ['bottom','right']){
 const frame=fanFrame({left:0,top:0,width:390,height,touch:44,gap:12,originX:350,originY:752,placement});
 const origin={x:350,y:752},reach=DECK_ACTION_REACH[placement];let previous=null,stretched=false,joined=false;
 for(let i=0;i<=200;i++){
  const travel=44*(reach.overview-.9+1.3*i/200),d=placement==='right'?travel*Math.SQRT1_2:0;
  const state={origin,point:{x:origin.x-d,y:origin.y-(placement==='right'?d:travel)},cancelArmed:false};
  const shape=fanActionMotion(frame,state).shape;
  if(shape.pull>1&&!shape.joined)stretched=true;
  if(shape.joined)joined=true;
  if(previous){assert(Math.abs(shape.pull-previous.pull)<1);assert(Math.abs(shape.neck-previous.neck)<4,'Pinching and growing the waist stays continuous');}
  assert(!/NaN|Infinity/.test(shape.path));previous=shape;
 }
 assert(stretched&&joined,'Both short horizontal and normal vertical capsules stretch before joining');checks++;
}
console.log(`${checks} action separation, reversible glass deformation, shared contour and viewport bounds checks passed.`);
