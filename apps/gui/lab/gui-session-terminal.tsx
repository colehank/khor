/** Shared titled presentation; bottom and expanded pills keep production dragging. */
import {useLayoutEffect,useState} from 'react';
import {DeckExplore} from '@/components/DeckExplore';
import {useCoarse} from '@/hooks/use-coarse';
import {FanLayer} from './gui-session-fan';
import {fanFrame} from './gui-session-fan-geometry';
import type {FanState} from './gui-session-fan-state';
export function useLabTerminalCoarse(){const coarse=useCoarse();return coarse||new URLSearchParams(location.search).get('phone')==='1';}
type Props=Parameters<typeof DeckExplore>[0];
function PillExplore({band,faces,state,origin,reach}:Props){
  const measure=()=>{
    const b=band.current!,r=b.getBoundingClientRect(),screen=b.closest('[data-session-screen]')!,host=screen.getBoundingClientRect(),head=screen.querySelector('[data-lab-model-header]')?.getBoundingClientRect(),v=window.visualViewport,css=getComputedStyle(b);
    const left=Math.max(host.left,v?.offsetLeft??0),top=Math.max(head?.bottom??host.top,v?.offsetTop??0),width=Math.min(host.right,(v?.offsetLeft??0)+(v?.width??innerWidth))-left;
    return fanFrame({left,top,width,height:Math.max(0,r.top-top),touch:parseFloat(css.getPropertyValue('--touch')),gap:parseFloat(css.getPropertyValue('--gap-row')),originX:origin,originY:r.y+r.height/2,placement:'bottom'});
  };
  const [frame,setFrame]=useState(measure);
  useLayoutEffect(()=>{const update=()=>setFrame(measure()),observer=new ResizeObserver(update);observer.observe(band.current!.closest('[data-session-screen]')!);window.addEventListener('resize',update);window.visualViewport?.addEventListener('resize',update);return()=>{observer.disconnect();window.removeEventListener('resize',update);window.visualViewport?.removeEventListener('resize',update);};},[band]);
  const start={x:origin,y:frame.originY};
  const fan:FanState={phase:state.overview?'overview':'fan',lane:state.overview||state.phase==='pull'?'overview':'fan',held:true,origin:start,point:{x:start.x+state.dx,y:start.y+state.dy},ids:faces.map(f=>f.id),hot:state.preview,armed:state.overview,direction:state.direction,tier:state.tier,interval:state.interval,pulse:state.pulse,deflection:Math.max(-1,Math.min(1,state.dx/reach)),acquired:true};
  return <FanLayer faces={faces} state={fan} frame={frame} phase={fan.phase} duration={parseFloat(getComputedStyle(band.current!).getPropertyValue('--dur-120'))}/>;
}
export function LabDeckExplore(props:Props){
  const enabled=new URLSearchParams(location.search).get('deck')==='fan'&&!!props.band.current?.closest('[data-session-bar],[data-lab-session-pill]');
  return enabled?<PillExplore {...props}/>:<DeckExplore {...props}/>;
}
