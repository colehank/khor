import type {CSSProperties} from 'react';
import {IconChevronUp,IconPlus} from '@/components/icons';
import {gui} from '@/gen/catalog';
import type {FanFrame} from './geometry';
import type {FanState} from './state';
import {fanActionMotion} from './action-motion';

export function FanActions({frame,state,create}:{frame:FanFrame;state:FanState;create:boolean}){
  const motion=fanActionMotion(frame,state,create);
  const style=(cell:typeof motion.newTab):CSSProperties=>({left:cell.x,top:cell.y-cell.height/2,width:cell.width,height:cell.height,'--fan-grow':cell.grow} as CSSProperties);
  return <div className="lab-fan-actions" data-lab-fan-actions data-transfer={motion.transfer} aria-hidden="true">
    <svg className="lab-fan-bond" width={frame.width} height={frame.height} data-lab-fan-bond data-connected={motion.opacity>.1||undefined} style={{opacity:motion.opacity}}><path d={motion.bridge}/></svg>
    {create&&<div data-lab-fan-new data-active={state.intent==='new'||undefined} data-grow={motion.newTab.grow} className="lab-fan-overview lab-fan-create glass absolute" style={style(motion.newTab)}><IconPlus/><span>{gui.new_tab}</span></div>}
    <div data-lab-fan-overview data-active={state.intent==='overview'||undefined} data-grow={motion.overview.grow} className="lab-fan-overview glass absolute" style={style(motion.overview)}><IconChevronUp centred/><span>全部标签页</span></div>
  </div>;
}
