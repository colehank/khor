import {deckControl} from '../../lib/deck-intent';
import {outerPoint,type FanFrame} from './geometry';
import type {FanState} from './state';
import {actionShapeBounds,liquidActionShape,type ActionCell} from './action-shape';

type Contact=Pick<FanState,'origin'|'point'|'cancelArmed'>&Partial<Pick<FanState,'intent'|'cursor'|'deflection'>>;
type Point={x:number;y:number};
const mix=(a:number,b:number,t:number)=>a+(b-a)*t;
const cubic=(a:number,b:number,c:number,d:number,t:number)=>(1-t)**3*a+3*(1-t)**2*t*b+3*(1-t)*t*t*c+t**3*d;

/** Two independent lenses approach, exchange size, and release their short neck.
 * No filter, timer or selection state is involved: withdrawing retraces the shape. */
export function fanActionMotion(frame:FanFrame,state:Contact,create=true){
  const touch=frame.touch;
  const control=deckControl({dx:state.origin.x-state.point.x,up:state.origin.y-state.point.y,touch,placement:frame.placement,previous:state.intent??'switch',homeDistance:state.cancelArmed?0:undefined},create);
  const ready=control.entry,transfer=control.transfer;
  const steering=Math.max(-2,Math.min(2,state.cursor??(state.deflection??0)*2));
  const arc=outerPoint(frame,steering*frame.guideExtent/2);
  const growth={create:create?ready*(1-transfer):0,overview:create?ready*transfer:transfer};
  const base=touch*.8;
  const cell=(target:{x:number;y:number},grow:number):ActionCell=>({x:target.x,y:target.y+base/2,width:touch*(.8+2.2*grow),height:touch*(.8+.18*grow),grow});
  const overview=cell(frame.target,growth.overview),newTab=cell(frame.createTarget,growth.create);
  const dx=newTab.x-overview.x,dy=newTab.y-overview.y,distance=Math.hypot(dx,dy),nx=distance?dx/distance:0,ny=distance?dy/distance:1;
  const vertical=Math.abs(ny)>=Math.abs(nx);
  const support=(c:ActionCell)=>vertical?c.height/2:c.width/2;
  const together=create?4*transfer*(1-transfer)*ready:0;
  const approach=Math.min(touch*.75,Math.max(0,(distance-support(overview)-support(newTab)-touch*.32)/2))*together;
  overview.x+=nx*approach;overview.y+=ny*approach;newTab.x-=nx*approach;newTab.y-=ny*approach;
  for(const c of [overview,newTab]){
    c.x=Math.max(c.width/2,Math.min(frame.width-c.width/2,c.x));
    c.y=Math.max(c.height/2,Math.min(frame.height-c.height/2,c.y));
  }
  let shape=liquidActionShape(overview,create?newTab:null,together,touch);
  const progress=control.progress,target=create?newTab:overview;
  let point:Point=progress===0?arc:{x:target.x,y:target.y+target.height/2};
  let pointOpacity=progress===0?1:0;
  const entering=progress>0&&progress<1&&(!create||transfer===0);
  if(entering){
    const pulse=4*progress*(1-progress),size=touch*.5*Math.sin(Math.PI*progress);
    target.y=Math.max(target.height/2,target.y-touch*.3*pulse);
    const end={x:target.x,y:target.y+target.height/2+size/2+touch*.08*(1-progress)};
    const travel=1-(1-progress)**2;
    if(frame.placement==='right'){
      // Follow the outside of the candidate arc before curving into the lens.
      const edge=Math.max(arc.x,end.x),bow=edge+Math.max(0,(frame.width-frame.gap-touch*.25-edge)/.75);
      point={x:cubic(arc.x,bow,bow,end.x,travel),y:cubic(arc.y,arc.y,end.y,end.y,travel)};
    }else point={x:mix(arc.x,end.x,1-(1-progress)**4),y:mix(arc.y,end.y,travel)};
    point={x:Math.max(size/2,Math.min(frame.width-size/2,point.x)),y:Math.max(size/2,Math.min(frame.height-size/2,point.y))};
    pointOpacity=1-progress**4;
    const seed:ActionCell={...point,width:size,height:size,grow:0};
    const cells=create?[overview,newTab,seed]:[overview,seed],bounds=actionShapeBounds(cells,touch);
    const tail=liquidActionShape(target,seed,pulse,touch,bounds);
    shape=create?{...tail,path:liquidActionShape(overview,null,0,touch,bounds).path+' '+tail.path}:tail;
  }else if(create&&transfer>0){
    point={x:mix(newTab.x,overview.x,transfer),y:mix(newTab.y-newTab.height/2,overview.y+overview.height/2,transfer)};
    pointOpacity=transfer===1?0:Math.sin(Math.PI*transfer);
  }
  return {newTab,overview,entry:ready,entering,transfer,shape,control,arc,point,pointOpacity};
}
