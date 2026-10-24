/** The approved shallow U, shared by root sessions, files and webpages. */
import {useLayoutEffect,useMemo,useRef,useState,type RefObject,type CSSProperties} from 'react';
import {createPortal} from 'react-dom';
import {TabAvatar,type TabFace} from '@/browse/TabStrip';
import {IconChevronUp,IconChevronRight} from '@/components/icons';
import {gui} from '@/gen/catalog';
import {nearbySlots} from '@/lib/deck-scrub';
import type {DeckJoystickState} from '@/lib/deck-joystick';
import {stillMotion} from '@/lib/travel';

/** Presentation follows each selected item around one continuous arc. It
 * never delays the engine's selection, pause, reversal or release. */
export function useDeckArcPosition(preview:number,count:number,direction:number,duration:number,direct=false,loop=true){
  const [position,setPosition]=useState(preview),current=useRef(preview),target=useRef(preview),previous=useRef(preview);
  useLayoutEffect(()=>{
    let delta=preview-previous.current;
    if(loop){if(delta>count/2)delta-=count;if(delta<-count/2)delta+=count;}
    if(loop&&delta&&direction&&Math.sign(delta)!==direction)delta+=direction*count;
    previous.current=preview;target.current+=delta;
    if(direct||stillMotion()){current.current=target.current;setPosition(target.current);return;}
    const from=current.current,to=target.current,began=performance.now();let frame=0;
    const tick=(now:number)=>{const p=Math.min(1,(now-began)/duration);current.current=from+(to-from)*(1-(1-p)**3);setPosition(current.current);if(p<1)frame=requestAnimationFrame(tick);};
    frame=requestAnimationFrame(tick);return()=>cancelAnimationFrame(frame);
  },[preview,count,direct,loop,duration]);
  return position;
}

export function DeckRateCue({side,state,x}:{side:number;state:Pick<DeckJoystickState,'direction'|'overview'|'tier'|'interval'|'pulse'>;x:number}){
  const active=state.direction===side&&!state.overview,level=active?state.tier:0;
  return <span className="deck-selector-rate" data-rate-side={side} data-active={active||undefined} data-rate-level={level}
    style={{left:x,'--cue-interval':`${state.interval}ms`} as CSSProperties} aria-hidden="true">
    <span className="deck-selector-rate-glyphs" key={`${active}-${state.pulse}`} style={{transform:side<0?'rotate(180deg)':undefined}}>
      {[0,1,2].map(i=><span key={i} data-lit={i<level||undefined} data-rest={level===0&&i===0||undefined} style={{'--cue-order':i} as CSSProperties}><IconChevronRight/></span>)}
    </span>
  </span>;
}

export function DeckExplore({band,faces,state,origin,reach,overview,placement='bottom'}:{band:RefObject<HTMLDivElement|null>;faces:TabFace[];state:DeckJoystickState;origin:number;reach:number;overview:boolean;placement?:'top'|'bottom'}){
  const measure=()=>{
    const b=band.current!,r=b.getBoundingClientRect(),viewport=window.visualViewport;
    const host=b.closest('[data-deck-explore-host]')?.getBoundingClientRect();
    const left=host?.left??viewport?.offsetLeft??0,top=host?.top??viewport?.offsetTop??0;
    const bottom=top+(host?.height??viewport?.height??window.innerHeight);
    return {left,top:placement==='top'?r.bottom:top,width:host?.width??viewport?.width??document.documentElement.clientWidth,height:Math.max(0,placement==='top'?bottom-r.bottom:Math.min(r.top-top,bottom-top))};
  };
  const [frame,setFrame]=useState(measure);
  useLayoutEffect(()=>{
    const update=()=>setFrame(measure()),observer=new ResizeObserver(update);update();
    for(let parent:HTMLElement|null=band.current;parent;parent=parent.parentElement)observer.observe(parent);
    window.addEventListener('resize',update);window.addEventListener('scroll',update,true);window.visualViewport?.addEventListener('resize',update);window.visualViewport?.addEventListener('scroll',update);
    return()=>{observer.disconnect();window.removeEventListener('resize',update);window.removeEventListener('scroll',update,true);window.visualViewport?.removeEventListener('resize',update);window.visualViewport?.removeEventListener('scroll',update);};
  },[band,placement]);
  const cs=getComputedStyle(band.current!),unit=parseFloat(cs.getPropertyValue('--touch'))||44;
  const duration=parseFloat(cs.getPropertyValue('--dur-180'))||180;
  const position=useDeckArcPosition(state.preview,faces.length,state.direction,duration);
  const radius=Math.min(205,frame.width*.46),f=faces[state.preview];
  const titleSegments=useMemo(()=>Array.from(new Intl.Segmenter('zh',{granularity:'word'}).segment(f?.label||''),piece=>piece.segment),[f?.label]);
  const labelWidth=Math.min(2*unit,frame.width*.24);
  const originX=origin-frame.left,pulling=state.phase==='pull',paused=state.direction===0&&!state.overview;
  // Small sets contain each identity once, including the two-tab case.
  const candidates=nearbySlots(position,faces.length,3).filter(s=>Math.abs(s.slot-position)<2.5);
  const unique=new Map<number,(typeof candidates)[number]>();
  for(const slot of candidates){const old=unique.get(slot.index);if(!old||Math.abs(slot.slot-position)<Math.abs(old.slot-position))unique.set(slot.index,slot);}
  return createPortal(<div className="deck-selector" data-deck-selector data-below={placement==='top'||undefined} data-sheet-hold data-overview={state.overview||undefined} data-direct-pull={pulling||undefined} style={{...frame,'--pull-progress':Math.min(1,Math.max(.15,-state.dy/48))} as CSSProperties}>
    <div className="deck-selector-dimmer"/>
    {!pulling&&<div className="deck-selector-wheel" data-deck-arc aria-hidden="true">
      {[...unique.values()].map(({slot,index})=>{
        const item=faces[index];if(!item)return null;
        const distance=slot-position,focus=Math.exp(-distance*distance*.9),edge=Math.min(1,(2.5-Math.abs(distance))/.5);
        // The current app's shallow U: center low, neighbors raised.
        const absolute=Math.abs(distance);
        // Three readable identities; peripheral faces only suggest continuity.
        const labelOpacity=Math.max(0,Math.min(1,(absolute-.8)/.2))*Math.max(0,Math.min(1,(1.7-absolute)/.35));
        const x=Math.sin(distance*.4)*radius+Math.sign(distance)*28*(1-Math.exp(-distance*distance*3));
        const y=-((1-Math.cos(distance*.4))*radius+12*(1-Math.exp(-distance*distance*3)));
        const labelShift=Math.max(-frame.width/2+labelWidth/2+4,Math.min(frame.width/2-labelWidth/2-4,x))-x;
        return <div key={slot} className="deck-selector-item" data-arc-offset={slot-Math.round(position)} data-current={index===state.preview||undefined}
          style={{transform:`translate3d(${x}px,${y}px,0)`,opacity:edge}}>
          <span className="deck-selector-face" style={{transform:`scale(${.78+.54*focus})`,opacity:.26+.74*focus}}>{!item.hideFace&&<TabAvatar f={item} bare="size-list-face" seat="row"/>}</span>
          <span className="deck-selector-label" style={{width:labelWidth,transform:`translateX(calc(-50% + ${labelShift}px))`,opacity:labelOpacity}}>{item.label}</span>
        </div>;
      })}
      <div className="deck-selector-title-seat" data-arc-title><span key={f?.id} className="deck-selector-title">{titleSegments.map((segment,i)=>/\p{Script=Han}/u.test(segment)?<span key={i} data-title-word>{segment}</span>:segment)}</span></div>
    </div>}
    {overview&&<div className="deck-selector-target" data-deck-overview-target data-active={state.overview||undefined} style={{left:originX}}><IconChevronUp centred/><span>{gui.deck_all_tabs}</span></div>}
    {!pulling&&<div className="deck-selector-steering-cues" aria-hidden="true">
      <DeckRateCue side={-1} state={state} x={Math.max(unit*.6,originX-reach-unit*.45)}/>
      <span className="deck-selector-origin" data-paused={paused||undefined} style={{left:originX}}/>
      <DeckRateCue side={1} state={state} x={Math.min(frame.width-unit*.6,originX+reach+unit*.45)}/>
    </div>}
    <div className="sr-only" role="status" aria-live="polite">{state.overview?gui.deck_all_tabs:pulling?(placement==='top'?gui.deck_gesture_down:gui.deck_gesture):gui.deck_preview(f?.label||'')}</div>
  </div>,document.body);
}
