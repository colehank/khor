import assert from 'node:assert/strict';
import {fanFrame,fanPoint,outerPoint,fanScale} from './gui-session-fan-geometry.ts';
let checks=0;
for(const placement of ['right','bottom'])for(const width of [320,390,430])for(const height of [390,700]){
 const frame=fanFrame({left:0,top:0,width,height,touch:44,gap:12,originX:width-36,originY:height+24,placement}),boxes=[];
 for(const offset of [-2,-1,0,1,2]){
  const p=fanPoint(frame,offset),s=fanScale(offset),o=outerPoint(frame,offset),left=p.x-(placement==='bottom'?frame.titleWidth:frame.face)*s/2,right=placement==='bottom'?p.x+frame.titleWidth*s/2:p.x+(frame.face/2+frame.gap+frame.titleWidth)*s,top=p.y-frame.face*s/2,bottom=p.y+(placement==='bottom'?frame.face/2+frame.touch*1.1:frame.face/2)*s;
  assert(left>=-1&&right<=width+1&&top>=-1&&bottom<=height+1,`${placement} ${width} ${height} ${offset} stays in the visible region`);
  if(placement==='right')assert(o.x<left-frame.gap,'right-hand control sits left of the entire identity');else assert(o.y<top-frame.gap,'bottom control sits above every identity');checks++;
  boxes.push({left,right,top,bottom});
 }
 for(let i=0;i<boxes.length;i++)for(let j=i+1;j<boxes.length;j++){const a=boxes[i],b=boxes[j];assert(a.right<=b.left||b.right<=a.left||a.bottom<=b.top||b.bottom<=a.top,`${placement} identities and their titles do not cover each other`);}checks++;
 for(let t=-2;t<=2;t+=.1){const p=outerPoint(frame,t);assert(p.x>=0&&p.x<=width&&p.y>=0&&p.y<=height,'the control path stays visible');for(const box of boxes)assert(p.x<box.left||p.x>box.right||p.y<box.top||p.y>box.bottom,'the whole control path stays outside every identity');}checks++;
 assert(fanScale(0)>fanScale(1)&&fanScale(1)>fanScale(2));checks++;
}
for(const width of [320,390,430]){
 const f=fanFrame({left:0,top:0,width,height:390,touch:44,gap:12,originX:width-36,originY:414,placement:'right'});
 const avatar=t=>fanPoint(f,t).x+(f.gap+f.titleWidth)*fanScale(t);
 for(const t of [1,2]){assert(Math.abs(avatar(t)-avatar(-t))<.001,'avatar arc has matching upper/lower ends');assert(Math.abs(outerPoint(f,t).x-outerPoint(f,-t).x)<.001,'control arc has matching upper/lower ends');assert(avatar(0)<avatar(t)&&outerPoint(f,0).x<outerPoint(f,t).x,'both arcs bow left like an opening parenthesis');checks++;}
 assert(avatar(0)<width-f.gap-f.face/2,'right-hand group moves left from the previous baseline');checks++;
}
console.log(`${checks} outer-control placement, five titled identity bounds and non-overlap checks passed.`);
