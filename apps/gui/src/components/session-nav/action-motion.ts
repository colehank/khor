import {deckActionTravel,DECK_ACTION_REACH} from '../../lib/deck-intent';
import type {FanFrame} from './geometry';
import type {FanState} from './state';
import {actionShapeBounds,liquidActionShape,type ActionCell} from './action-shape';

type Contact=Pick<FanState,'origin'|'point'|'cancelArmed'>;
const unit=(value:number)=>Math.max(0,Math.min(1,Number.isFinite(value)?value:0));
const ease=(value:number)=>{const t=unit(value);return t*t*(3-2*t);};

/** Two independent lenses approach, exchange size, and release their short neck.
 * No filter, timer or selection state is involved: withdrawing retraces the shape. */
export function fanActionMotion(frame:FanFrame,state:Contact,create=true){
  const touch=frame.touch,reach=DECK_ACTION_REACH[frame.placement];
  const travel=state.cancelArmed?0:deckActionTravel(state.origin.x-state.point.x,state.origin.y-state.point.y,frame.placement)/touch;
  const ready=ease((travel-reach.create+1.2)/1.2);
  const transfer=ease((travel-reach.overview+.9)/1.3);
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
  const entering=create&&ready>0&&ready<1&&transfer===0;
  if(entering){
    // The first handoff pulls New out of the candidate zone using the same
    // stretching caps and pinched waist as the later New / All exchange.
    const pulse=4*ready*(1-ready),size=touch*.5*Math.sin(Math.PI*ready);
    newTab.y=Math.min(frame.height-newTab.height/2,newTab.y+touch*.15*pulse);
    const seed:ActionCell={x:newTab.x,y:Math.min(frame.height-size/2,newTab.y+touch*(.65+.25*(1-ready))),width:size,height:size,grow:0};
    const bounds=actionShapeBounds([overview,newTab,seed],touch);
    const tail=liquidActionShape(newTab,seed,pulse,touch,bounds),other=liquidActionShape(overview,null,0,touch,bounds);
    shape={...tail,path:other.path+' '+tail.path};
  }
  return {newTab,overview,entry:ready,entering,transfer,shape};
}
