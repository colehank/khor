import {useLayoutEffect,useState} from 'react';
import {DeckExplore as DefaultExplore} from './DeckExplore';
import {FanLayer} from './session-nav/fan';
import {fanFrame} from './session-nav/geometry';
import type {FanState} from './session-nav/state';
type Props=Parameters<typeof DefaultExplore>[0];
function exploreHost(band:HTMLElement){return band.closest<HTMLElement>('[data-session-screen]')??band.closest<HTMLElement>('[data-file-page],[data-deck-explore-host]')!;}
function PillExplore({band,faces,state,origin,reach,create}:Props){
  const measure=()=>{
    const b=band.current!,r=b.getBoundingClientRect(),screen=exploreHost(b),host=screen.getBoundingClientRect(),head=screen.querySelector('[data-lab-model-header]')?.getBoundingClientRect(),v=window.visualViewport,css=getComputedStyle(b);
    const left=Math.max(host.left,v?.offsetLeft??0),top=Math.max(head?.bottom??host.top,v?.offsetTop??0),width=Math.min(host.right,(v?.offsetLeft??0)+(v?.width??innerWidth))-left;
    return fanFrame({left,top,width,height:Math.max(0,r.top-top),touch:parseFloat(css.getPropertyValue('--touch')),gap:parseFloat(css.getPropertyValue('--gap-row')),originX:origin-left,originY:r.y+r.height/2-top,placement:'bottom'});
  };
  const [frame,setFrame]=useState(measure);
  useLayoutEffect(()=>{const update=()=>setFrame(measure()),observer=new ResizeObserver(update);observer.observe(exploreHost(band.current!));window.addEventListener('resize',update);window.visualViewport?.addEventListener('resize',update);return()=>{observer.disconnect();window.removeEventListener('resize',update);window.visualViewport?.removeEventListener('resize',update);};},[band]);
  const start={x:frame.originX,y:frame.originY};
  const action=state.intent==='new'?'new':state.overview?'overview':'fan';
  const fan:FanState={phase:action,lane:action,held:true,origin:start,point:{x:start.x+state.dx,y:start.y+state.dy},ids:faces.map(f=>f.id),hot:state.preview,position:state.position,target:state.target,armed:state.overview,cancelArmed:state.intent==='keep',direction:state.direction,tier:state.tier,interval:state.interval,pulse:state.pulse,deflection:Math.max(-1,Math.min(1,state.dx/reach)),acquired:true,opened:state.opened,intent:state.intent};
  return <FanLayer faces={faces} state={fan} frame={frame} phase={fan.phase} create={create} duration={parseFloat(getComputedStyle(band.current!).getPropertyValue('--dur-120'))}/>;
}
export function DeckExplore(props:Props){
  const screen=props.band.current&&exploreHost(props.band.current);
  const enabled=props.placement!=='top'&&(!!screen||!!props.band.current?.closest('[data-lab-session-pill]'));
  return enabled?<PillExplore {...props}/>:<DefaultExplore {...props}/>;
}
