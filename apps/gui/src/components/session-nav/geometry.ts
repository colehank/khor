import type {FanPoint} from './state';
export type FanPlacement='right'|'bottom';
export type FanFrame={left:number;top:number;width:number;height:number;placement:FanPlacement;compact:boolean;horizontal:boolean;slots:number;anchor:FanPoint;touch:number;face:number;gap:number;pitch:number;curve:number;titleWidth:number;originX:number;originY:number;target:FanPoint;createTarget:FanPoint;guideExtent:number};
type Bounds={left:number;top:number;width:number;height:number;touch:number;gap:number;originX:number;originY:number;placement?:FanPlacement};
export const fanScale=(offset:number)=>.6+.4*Math.exp(-offset*offset*.9);
function rightSweep(frame:FanFrame,offset:number){
  // Tilt the opening arc: the lower return stroke goes inward, while
  // its upper end keeps the same right-hand reach as the previous curve.
  return {x:frame.anchor.x+frame.curve*(.45*offset*offset+1.1*offset),y:frame.anchor.y-offset*frame.pitch};
}
export function fanPoint(frame:FanFrame,offset:number){
  if(frame.placement==='bottom'){
    // Equal horizontal seats keep adjacent titles apart throughout a drag.
    const center=frame.width/2+offset*(frame.width-2*frame.gap)/3;
    return {x:center,y:frame.anchor.y-frame.pitch*(Math.sqrt(offset*offset+.1)-Math.sqrt(.1))};
  }
  if(frame.horizontal)return {x:frame.anchor.x+offset*frame.pitch,y:frame.anchor.y};
  // Position the avatar on the sweep independently of its title's scale.
  const p=rightSweep(frame,offset);
  return {x:p.x-(frame.gap+frame.titleWidth)*fanScale(offset),y:p.y};
}
export function outerPoint(frame:FanFrame,offset:number){
  if(frame.placement==='bottom'){
    const edge=(frame.slots-1)/2,top=fanPoint(frame,edge).y-frame.face*fanScale(edge)/2;
    return {x:frame.width/2+offset*Math.min(1.1*frame.touch,(frame.width/2-frame.gap)/2),y:top-frame.gap-frame.touch*.28-offset*offset*frame.touch*.25};
  }
  if(!frame.horizontal){
    const p=rightSweep(frame,offset);
    return {x:p.x+frame.face/2+frame.gap+frame.touch*.28,y:p.y};
  }
  const p=fanPoint(frame,offset),scale=fanScale(offset),space=frame.face*scale/2+frame.gap+frame.touch*.28;
  return frame.horizontal?{x:p.x+(frame.gap+frame.titleWidth)/2*scale,y:p.y-space}:{x:p.x-space,y:p.y};
}
export function fanFrame(b:Bounds):FanFrame{
  const placement=b.placement??'right',{width,height,touch,gap}=b,face=touch*1.32;
  const actionHeight=touch*.8,rowActions=height<(placement==='bottom'?6.5:5)*touch,room=height-(rowActions?actionHeight+2*gap:2*actionHeight+3*gap);
  const compact=room<6.8*touch+2*gap,horizontal=placement==='right'&&compact&&width>=12*touch,slots=placement==='bottom'?(room>=4.5*touch?3:1):compact?(horizontal||room>=4.5*touch?3:1):5;
  const row=placement==='bottom'?Math.min(3*touch,(width-2*gap)/3):0;
  const titleWidth=placement==='bottom'?row:horizontal?3*touch:3.4*touch;
  const pitch=placement==='bottom'?touch*.8:horizontal?(width-2*gap-face-gap-titleWidth)/(slots-1):Math.min(1.5*touch,(height-2*gap-face*(slots===3 ? .76 : .61))/Math.max(1,slots-1));
  // Reserve the right edge for the thumb guide; fit the entire title to
  // its left, flattening the curve when a narrow viewport needs room.
  const edge=(slots-1)/2,rightEdge=width-2*gap-face/2-touch*(.3+.28);
  // Keep the same inward seat even when a short viewport shows fewer items.
  const curve=Math.max(0,Math.min(touch*.45,(rightEdge-titleWidth-face/2-2*gap)/4));
  const anchor={x:placement==='bottom'?width/2:horizontal?gap+face/2+pitch*(slots-1)/2:rightEdge-curve*4,y:placement==='bottom'?Math.max((rowActions?actionHeight:0)+gap+face/2,height-gap-face/2-touch*1.65):horizontal||slots===1?Math.max(face/2,Math.min(height-face/2,height*.62)):height-gap-face*(slots===3 ? .76 : .61)/2-(slots-1)/2*pitch};
  if(placement==='right'&&!horizontal){
    anchor.x-=Math.min(touch*.3,Math.max(0,anchor.x-titleWidth-face/2-gap*1.5));
    anchor.y-=Math.min(touch*.7,Math.max(0,anchor.y-edge*pitch-face*fanScale(edge)/2-gap));
  }
  const guideExtent=slots===1 ? .7 : placement==='right'&&!horizontal?(slots-1)/2:2;
  const frame:FanFrame={...b,placement,compact,horizontal,slots,face,titleWidth,pitch,curve,anchor,guideExtent,target:{x:0,y:0},createTarget:{x:0,y:0}};
  // Actions and the thumb guide share the same arc geometry, outside its candidates.
  const guideTop=Math.min(...Array.from({length:41},(_,i)=>outerPoint(frame,guideExtent*(-1+i/20)).y))-touch*.28;
  const enteringEdge=slots/2;
  const candidateTop=Math.min(...[-enteringEdge,0,enteringEdge].map(offset=>fanPoint(frame,offset).y-face*fanScale(offset)/2));
  if(rowActions){
    frame.createTarget={x:width/3,y:gap/4};frame.target={x:2*width/3,y:gap/4};
  }else{
    const actionHalf=Math.min(width/2-gap,1.7*touch);
    const x=Math.max(actionHalf+gap,Math.min(width-actionHalf-gap,outerPoint(frame,0).x));
    const y=Math.max(gap,(placement==='bottom'?candidateTop:Math.min(guideTop,candidateTop))-2*gap-2*actionHeight);
    frame.target={x,y};frame.createTarget={x,y:y+actionHeight+gap};
  }
  return frame;
}
