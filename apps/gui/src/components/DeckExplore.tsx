/** Long-press exploration shares the pill's identities, never opens a preview. */
import {useLayoutEffect,useRef,useState,type RefObject} from 'react';
import {createPortal} from 'react-dom';
import {TabAvatar,type TabFace} from '@/browse/TabStrip';
import {nearbySlots} from '@/lib/deck-scrub';

type Frame={left:number;top:number;width:number;height:number;center:number};
export function DeckExplore({band,faces,position,preview,leaving}: {
  band:RefObject<HTMLDivElement|null>;faces:TabFace[];position:number;preview:number;leaving:boolean;
}) {
  const title=useRef<HTMLSpanElement>(null),titleSeat=useRef<HTMLDivElement>(null);
  const measure=():Frame=>{
    const b=band.current;if(!b)return {left:0,top:0,width:0,height:0,center:0};
    const r=b.getBoundingClientRect(),viewport=window.visualViewport;
    // The explicit host only bounds component fixtures. The app uses its
    // visible viewport; the deck itself already sits above the keyboard.
    const host=b.closest('[data-deck-explore-host]')?.getBoundingClientRect();
    const left=host?.left??viewport?.offsetLeft??0,top=host?.top??viewport?.offsetTop??0;
    const width=host?.width??viewport?.width??document.documentElement.clientWidth;
    const height=Math.max(0,Math.min(r.top-top,host?.height??viewport?.height??window.innerHeight));
    const cs=getComputedStyle(b),face=parseFloat(cs.getPropertyValue('--glass-btn'))||44;
    const line=title.current?parseFloat(getComputedStyle(title.current).lineHeight):22;
    const gap=parseFloat(cs.getPropertyValue('--gap-base'))||8;
    const center=Math.max(0,Math.min(Math.max(116,height*.61),height-face-2*line-3*gap));
    return {left,top,width,height,center};
  };
  const [frame,setFrame]=useState(measure);
  useLayoutEffect(()=>{
    const update=()=>setFrame(measure());update();
    const observer=new ResizeObserver(update);
    if(band.current)observer.observe(band.current);if(titleSeat.current)observer.observe(titleSeat.current);
    // Ancestor size changes also move a deck when a keyboard or sheet moves.
    for(let parent=band.current?.parentElement;parent;parent=parent.parentElement)observer.observe(parent);
    window.addEventListener('resize',update);window.addEventListener('scroll',update,true);window.visualViewport?.addEventListener('resize',update);window.visualViewport?.addEventListener('scroll',update);
    return()=>{observer.disconnect();window.removeEventListener('resize',update);window.removeEventListener('scroll',update,true);window.visualViewport?.removeEventListener('resize',update);window.visualViewport?.removeEventListener('scroll',update);};
  },[band]);
  const extent=Math.min(2.6,faces.length/2),radius=Math.min(205,frame.width*.46);
  const slots=nearbySlots(position,faces.length,3).filter(({slot})=>Math.abs(slot-position)<extent);
  return createPortal(<div data-deck-selector data-sheet-hold data-leaving={leaving||undefined}
    className="deck-selector" style={{left:frame.left,top:frame.top,width:frame.width,height:frame.height}}>
    <div className="deck-selector-dimmer"/>
    <div role="listbox" aria-label={faces[preview]?.label} className="deck-selector-wheel" style={{top:frame.center}}>
      {slots.map(({slot,index})=>{
        const distance=slot-position,absolute=Math.abs(distance),focus=Math.exp(-distance*distance*.9);
        const edge=Math.min(1,Math.max(0,(extent-absolute)/.42));
        const x=Math.sin(distance*.4)*radius+Math.sign(distance)*28*(1-Math.exp(-distance*distance*3));
        const y=-((1-Math.cos(distance*.4))*radius+12*(1-Math.exp(-distance*distance*3)));
        return <div key={slot} role="option" aria-label={faces[index].label} aria-selected={slot===Math.round(position)}
          className="deck-selector-item" style={{transform:`translate3d(${x}px,${y}px,0)`,opacity:edge}}>
          <span className="deck-selector-face" style={{transform:`scale(${.78+.22*focus})`,opacity:.64+.36*focus}}>
            {!faces[index].hideFace&&<TabAvatar f={faces[index]} bare="size-list-face" seat="row"/>}
          </span>
          <span className="deck-selector-label" aria-hidden="true" style={{opacity:.84*Math.max(0,Math.min(1,(absolute-.55)/.25))}}>{faces[index].label}</span>
        </div>;
      })}
      <div ref={titleSeat} className="deck-selector-title-seat" aria-hidden="true">
        <span ref={title} key={faces[preview]?.id} className="deck-selector-title">{faces[preview]?.label}</span>
      </div>
    </div>
  </div>,document.body);
}
