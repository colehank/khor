import {deckActionTravel,DECK_ACTION_REACH} from '../../lib/deck-intent';
import type {FanFrame} from './geometry';
import type {FanState} from './state';

type Cell={x:number;y:number;width:number;height:number;grow:number};
type Contact=Pick<FanState,'origin'|'point'|'cancelArmed'>;
const unit=(value:number)=>Math.max(0,Math.min(1,Number.isFinite(value)?value:0));
const ease=(value:number)=>{const t=unit(value);return t*t*(3-2*t);};
const point=(x:number,y:number)=>`${x} ${y}`;

/** Two independent lenses approach, exchange size, and release their short neck.
 * No filter, timer or selection state is involved: withdrawing retraces the shape. */
export function fanActionMotion(frame:FanFrame,state:Contact,create=true){
  const touch=frame.touch,reach=DECK_ACTION_REACH[frame.placement];
  const travel=state.cancelArmed?0:deckActionTravel(state.origin.x-state.point.x,state.origin.y-state.point.y,frame.placement)/touch;
  const ready=ease((travel-reach.create+.6)/.6);
  const transfer=ease((travel-reach.overview+.9)/1.3);
  const growth={create:create?ready*(1-transfer):0,overview:create?ready*transfer:transfer};
  const base=touch*.8;
  const cell=(target:{x:number;y:number},grow:number):Cell=>({x:target.x,y:target.y+base/2,width:touch*(.8+2.2*grow),height:touch*(.8+.18*grow),grow});
  const overview=cell(frame.target,growth.overview),newTab=cell(frame.createTarget,growth.create);
  const dx=newTab.x-overview.x,dy=newTab.y-overview.y,distance=Math.hypot(dx,dy),nx=distance?dx/distance:0,ny=distance?dy/distance:1;
  const vertical=Math.abs(ny)>=Math.abs(nx);
  const support=(c:Cell)=>vertical?c.height/2:c.width/2;
  const together=create?4*transfer*(1-transfer)*ready:0;
  const approach=Math.min(touch*.75,Math.max(0,(distance-support(overview)-support(newTab)-touch*.08)/2))*together;
  overview.x+=nx*approach;overview.y+=ny*approach;newTab.x-=nx*approach;newTab.y-=ny*approach;
  for(const c of [overview,newTab]){
    c.x=Math.max(c.width/2,Math.min(frame.width-c.width/2,c.x));
    c.y=Math.max(c.height/2,Math.min(frame.height-c.height/2,c.y));
  }
  const gap=distance-2*approach-support(overview)-support(newTab);
  const opacity=create?ease((touch*.4-gap)/(touch*.4))*together:0;
  let bridge='';
  if(opacity>.001){
    const px=-ny,py=nx,neck=Math.min(overview.height,newTab.height)*.27;
    const ax=overview.x+nx*(support(overview)-touch*.05),ay=overview.y+ny*(support(overview)-touch*.05);
    const bx=newTab.x-nx*(support(newTab)-touch*.05),by=newTab.y-ny*(support(newTab)-touch*.05);
    const midx=(ax+bx)/2,midy=(ay+by)/2;
    bridge=`M ${point(ax+px*neck,ay+py*neck)} C ${point(midx+px*neck*.3,midy+py*neck*.3)} ${point(midx+px*neck*.3,midy+py*neck*.3)} ${point(bx+px*neck,by+py*neck)} L ${point(bx-px*neck,by-py*neck)} C ${point(midx-px*neck*.3,midy-py*neck*.3)} ${point(midx-px*neck*.3,midy-py*neck*.3)} ${point(ax-px*neck,ay-py*neck)} Z`;
  }
  return {newTab,overview,transfer,bridge,opacity};
}
