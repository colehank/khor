import type {CSSProperties} from 'react';
import {IconChevronUp,IconPlus} from '@/components/icons';
import {gui} from '@/gen/catalog';
import type {FanFrame} from './geometry';
import type {FanState} from './state';
import {fanActionMotion} from './action-motion';

export function FanActions({frame,state,create}:{frame:FanFrame;state:FanState;create:boolean}){
  const motion=fanActionMotion(frame,state,create);
  const {shape}=motion;
  const surface={left:shape.left,top:shape.top,width:shape.width,height:shape.height};
  const clip=`path("${shape.path}")`;
  const style=(cell:typeof motion.newTab):CSSProperties=>({left:cell.x,top:cell.y-cell.height/2,width:cell.width,height:cell.height,'--fan-grow':cell.grow} as CSSProperties);
  return <div className="lab-fan-actions" data-lab-fan-actions data-entry={motion.entry} data-entering={motion.entering||undefined} data-transfer={motion.transfer} aria-hidden="true">
    <div data-lab-fan-surface className="lab-fan-surface glass absolute" style={{...surface,clipPath:clip,WebkitClipPath:clip}}/>
    <svg className="lab-fan-bond" width={shape.width} height={shape.height} data-lab-fan-bond data-connected={shape.joined||undefined} data-pull={shape.pull} style={surface}><path d={shape.path}/></svg>
    {create&&<div data-lab-fan-new data-active={state.intent==='new'||undefined} data-grow={motion.newTab.grow} className="lab-fan-overview lab-fan-create" style={style(motion.newTab)}><IconPlus/><span>{gui.new_tab}</span></div>}
    <div data-lab-fan-overview data-active={state.intent==='overview'||undefined} data-grow={motion.overview.grow} className="lab-fan-overview" style={style(motion.overview)}><IconChevronUp centred/><span>全部标签页</span></div>
  </div>;
}
