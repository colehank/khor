import assert from 'node:assert/strict';
import {fanFrame,fanPoint,outerPoint,fanScale} from '../../src/components/session-nav/geometry.ts';
let checks=0;
for(const placement of ['right','bottom'])for(const width of [320,390,430])for(const height of [390,700])for(const phase of [0,.25,.5,.75]){
 const frame=fanFrame({left:0,top:0,width,height,touch:44,gap:12,originX:width-36,originY:height+24,placement}),boxes=[];
 const edge=(frame.slots-1)/2;
 for(const offset of Array.from({length:frame.slots+1},(_,i)=>i-edge-phase).filter(offset=>Math.abs(offset)<frame.slots/2)){
  const p=fanPoint(frame,offset),s=fanScale(offset),o=outerPoint(frame,offset),left=p.x-(placement==='bottom'?frame.titleWidth:frame.face)*s/2,right=placement==='bottom'?p.x+frame.titleWidth*s/2:p.x+(frame.face/2+frame.gap+frame.titleWidth)*s,top=p.y-frame.face*s/2,bottom=p.y+(placement==='bottom'?frame.face/2+frame.touch*1.1:frame.face/2)*s;
  assert(left>=-1&&right<=width+1&&top>=-1&&bottom<=height+1,`${placement} ${width} ${height} ${offset} stays in the visible region`);
  if(placement==='right')assert(o.x>right+frame.gap,'right-hand control sits right of the entire identity');else assert(o.y<top-frame.gap,'bottom control sits above every identity');checks++;
  boxes.push({left,right,top,bottom});
 }
 for(let i=0;i<boxes.length;i++)for(let j=i+1;j<boxes.length;j++){const a=boxes[i],b=boxes[j];assert(a.right<=b.left||b.right<=a.left||a.bottom<=b.top||b.bottom<=a.top,`${placement} ${width}x${height} phase ${phase} identities overlap: ${JSON.stringify({a,b})}`);}checks++;
 for(let t=-frame.guideExtent;t<=frame.guideExtent;t+=.1){const p=outerPoint(frame,t);assert(p.x>=0&&p.x<=width&&p.y>=0&&p.y<=height,'the control path stays visible');for(const box of boxes)assert(p.x<box.left||p.x>box.right||p.y<box.top||p.y>box.bottom,'the whole control path stays outside every identity');}checks++;
 for(const target of [frame.target,frame.createTarget]){const action={left:target.x-frame.touch,right:target.x+frame.touch,top:target.y,bottom:target.y+frame.touch*.8};assert(action.left>=-1&&action.right<=width+1&&action.top>=0&&action.bottom<=height,'action prompts fit the same actual frame');for(const item of boxes)assert(action.right<=item.left||action.left>=item.right||action.bottom<=item.top||action.top>=item.bottom,'action prompts stay outside visible candidate identities');checks++;}
 assert(fanScale(0)>fanScale(1)&&fanScale(1)>fanScale(2));checks++;
}
for(const width of [320,390,430]){
 const f=fanFrame({left:0,top:0,width,height:390,touch:44,gap:12,originX:width-36,originY:414,placement:'right'});
 const avatar=t=>fanPoint(f,t).x+(f.gap+f.titleWidth)*fanScale(t);
 for(const t of new Set([Math.min(1,(f.slots-1)/2),(f.slots-1)/2])){assert(avatar(-t)<avatar(0)&&outerPoint(f,-t).x<outerPoint(f,0).x,'lower arc comes inward instead of returning to the right corner');assert((avatar(t)+avatar(-t))/2>avatar(0),'tilted avatar arc retains its opening-parenthesis curvature');checks++;}
 const lower=outerPoint(f,-(f.slots-1)/2);assert(lower.x<=f.originX-f.touch*.8&&lower.y<=f.height-f.touch*1.2,'lower thumb path reserves left and upward return room');checks++;
 assert(avatar(0)<width-f.gap-f.face/2,'right-hand group moves left from the previous baseline');checks++;
}
for(const width of [320,390,430])for(const height of [160,230,290]){
 const f=fanFrame({left:0,top:0,width,height,touch:44,gap:12,originX:width-36,originY:height+24,placement:'right'}),extent=f.guideExtent,lower=outerPoint(f,-extent);
 assert(lower.x<=f.originX-f.touch*.8&&lower.y<=f.originY-f.touch*1.2,'compact thumb path also stays inward and above its entry');
 for(let i=0;i<=40;i++){const p=outerPoint(f,extent*(-1+i/20));assert(p.x>=0&&p.x<=width&&p.y>=0&&p.y<=height,'shortened compact guide fits the available area');}checks++;
}
console.log(`${checks} outer-control placement, adaptive titled identity bounds and action separation, inward return and compact-guide checks passed.`);
