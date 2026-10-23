/** Shared mobile pill: ordinary direct swipe, held joystick, upward overview. */
import {useEffect,useLayoutEffect,useMemo,useRef,useState,type ReactNode,type PointerEvent} from 'react';
import {MidName,TabAvatar,type TabFace} from '@/browse/TabStrip';
import {BellMark} from '@/components/BellMark';
import {NoticeLook} from '@/components/PillNotice';
import {gui} from '@/gen/catalog';
import {cn} from '@/lib/utils';
import {feel} from '@/lib/haptics';
import {stillMotion} from '@/lib/travel';
import {deckScrubber,DEFAULT_DECK_TUNING,holdTiming,nearbySlots,type DeckScrubState} from '@/lib/deck-scrub';
import {deckJoystick,DECK_JOYSTICK_DEFAULTS,type DeckJoystickState,type DeckJoystickTuning} from '@/lib/deck-joystick';
import {DeckExplore} from './DeckExplore';
import './deck-band.css';

type Drag={pointer:number;touch:boolean;touchId:number|null;title:boolean;x:number;layout:string;route:'joystick'|'swipe'};
export function DeckBand({faces,active,onFocus,onTitle,onOverview,tuning=DECK_JOYSTICK_DEFAULTS,renderFace,placement='bottom'}: {
  faces:TabFace[];active:string;onFocus:(id:string)=>void;onTitle?:()=>void;onOverview?:()=>void;
  /** Isolated fixtures can tune one contact; the App uses the approved defaults. */
  tuning?:DeckJoystickTuning;
  renderFace?:(face:TabFace,role:'prev'|'cur'|'next')=>ReactNode;
  placement?:'top'|'bottom';
}) {
  const upward=placement==='bottom';
  const gesture=upward?gui.deck_gesture:gui.deck_gesture_down;
  const band=useRef<HTMLDivElement>(null),drag=useRef<Drag|null>(null),pendingTap=useRef(false);
  const latest=useRef({faces,active,onFocus,onTitle,onOverview,tuning});latest.current={faces,active,onFocus,onTitle,onOverview,tuning};
  const at=Math.max(0,faces.findIndex(f=>f.id===active));
  const [state,setState]=useState<DeckScrubState>({opened:at,preview:at,position:at,mode:'idle',source:'swipe',direction:0,holding:false,...holdTiming(0,250)});
  const [steering,setSteering]=useState<DeckJoystickState>({phase:'idle',preview:at,dx:0,dy:0,direction:0,interval:tuning.slowMs,overview:false,tier:0,pulse:0});
  const alive=useRef(true),prior=useRef(at);
  const layoutStamp=()=>{
    const r=band.current?.getBoundingClientRect(),v=window.visualViewport;
    return [r?.x,r?.y,r?.width,r?.height,innerWidth,innerHeight,v?.width,v?.height,v?.offsetLeft,v?.offsetTop].join(':');
  };
  const focus=(index:number)=>{const now=latest.current,f=now.faces[index];if(f&&f.id!==now.active)now.onFocus(f.id);};
  const engine=useMemo(()=>deckScrubber(faces.length,at,s=>{
    if(!alive.current)return;
    if(s.preview!==prior.current&&s.mode==='swipe')feel('threshold');
    prior.current=s.preview;setState(s);
  },focus,undefined,()=>({...DEFAULT_DECK_TUNING,longPressMs:Infinity})),[faces.length]);
  const steer=useMemo(()=>deckJoystick(faces.length,at,s=>{
    if(!alive.current)return;
    if(s.preview!==prior.current&&s.phase==='steering')feel('threshold');
    prior.current=s.preview;
    if((s.phase==='steering'||s.phase==='pull')&&engine.read().mode==='pending')engine.cancel();
    setSteering(s);
  },focus,()=>latest.current.onOverview?.(),undefined,()=>latest.current.tuning),[faces.length,at]);
  const clearCapture=()=>{const d=drag.current;drag.current=null;if(d&&band.current?.hasPointerCapture(d.pointer))band.current.releasePointerCapture(d.pointer);};
  const cancel=()=>{pendingTap.current=false;clearCapture();steer.cancel();engine.select(Math.max(0,latest.current.faces.findIndex(f=>f.id===latest.current.active)));};
  const ids=JSON.stringify(faces.map(f=>f.id));
  useLayoutEffect(()=>{cancel();},[engine,steer,active,ids,placement]);
  useEffect(()=>{alive.current=true;return()=>{alive.current=false;steer.dispose();engine.cancel();};},[engine,steer]);
  const finish=(id:number)=>{
    const d=drag.current;if(!d||id!==d.pointer)return;
    // Viewport metrics can change before its resize event is delivered.
    // Releasing in that interval must still cancel the old contact.
    if(d.layout!==layoutStamp()){cancel();return;}
    // A touch tap opens on its compatibility click. Opening on pointerup
    // would let that click hit the newly mounted sheet's scrim instead.
    clearCapture();
    if(d.route==='joystick')engine.cancel();
    pendingTap.current=(d.route==='swipe'?engine.release(stillMotion()):steer.release())==='tap'&&d.title;
  };
  useEffect(()=>{
    const b=band.current;if(!b)return;
    let width=b.clientWidth;
    const measure=()=>b.style.setProperty('--deck-band-width',`${b.clientWidth}px`);measure();
    const resize=new ResizeObserver(()=>{if(width!==b.clientWidth){cancel();width=b.clientWidth;measure();}});resize.observe(b);
    const blur=()=>cancel(),hidden=()=>{if(document.visibilityState!=='visible')cancel();};
    const viewport=()=>{if(drag.current)cancel();};
    const key=(e:KeyboardEvent)=>{if(e.key==='Escape'&&drag.current){cancel();e.preventDefault();e.stopPropagation();}};
    const multi=(e:globalThis.PointerEvent)=>{if(!e.isPrimary)cancel();};
    const start=(e:TouchEvent)=>{const d=drag.current;if(e.touches.length>1){cancel();return;}if(!d?.touch||d.touchId!==null)return;const t=Array.from(e.changedTouches).find(t=>t.target instanceof Node&&b.contains(t.target));if(t)d.touchId=t.identifier;};
    const end=(e:TouchEvent)=>{const d=drag.current;if(!d?.touch)return;const t=Array.from(e.changedTouches).find(t=>t.identifier===d.touchId);if(!t)return;if(e.type==='touchcancel')cancel();else finish(d.pointer);};
    const move=(e:TouchEvent)=>{if(drag.current&&(steer.read().phase==='steering'||steer.read().phase==='pull'))e.preventDefault();};
    window.addEventListener('touchstart',start,{capture:true,passive:true});window.addEventListener('touchend',end,{capture:true,passive:true});window.addEventListener('touchcancel',end,{capture:true,passive:true});window.addEventListener('touchmove',move,{passive:false,capture:true});
    window.addEventListener('pointerdown',multi,true);window.addEventListener('blur',blur);window.addEventListener('pagehide',blur);document.addEventListener('visibilitychange',hidden);window.addEventListener('keydown',key,true);
    window.addEventListener('resize',viewport);window.visualViewport?.addEventListener('resize',viewport);
    return()=>{resize.disconnect();window.removeEventListener('touchstart',start,true);window.removeEventListener('touchend',end,true);window.removeEventListener('touchcancel',end,true);window.removeEventListener('touchmove',move,true);window.removeEventListener('pointerdown',multi,true);window.removeEventListener('blur',blur);window.removeEventListener('pagehide',blur);document.removeEventListener('visibilitychange',hidden);window.removeEventListener('keydown',key,true);window.removeEventListener('resize',viewport);window.visualViewport?.removeEventListener('resize',viewport);clearCapture();};
  },[engine,steer]);
  const down=(e:PointerEvent)=>{
    if(!e.isPrimary){cancel();return;}if(e.button!==0||drag.current||!band.current)return;
    if(engine.read().mode==='settling'){engine.arrive();return;}
    if(e.target instanceof Element&&e.target.closest('input,textarea,[contenteditable=true],[data-band-control]'))return;
    if(window.getSelection()?.isCollapsed===false)return;
    pendingTap.current=false;
    drag.current={pointer:e.pointerId,touch:e.pointerType==='touch',touchId:null,title:!!(e.target instanceof Element&&e.target.closest('[data-band-seg="cur"]')),x:e.clientX,layout:layoutStamp(),route:'joystick'};
    // Both recognizers start at touch-down, preserving the old swipe's
    // velocity history. Only the joystick may claim a stationary hold.
    const cs=getComputedStyle(band.current),step=parseFloat(cs.getPropertyValue('--glass-btn'))+3*parseFloat(cs.getPropertyValue('--gap-pane'));
    engine.start(e.clientX,e.clientY,band.current.clientWidth,step);
    steer.start(e.clientX,upward?e.clientY:-e.clientY);e.preventDefault();band.current.setPointerCapture(e.pointerId);
  };
  const moving=(e:PointerEvent)=>{
    const d=drag.current;if(!d||d.pointer!==e.pointerId)return;
    if(d.route==='joystick'){
      steer.move(e.clientX,upward?e.clientY:-e.clientY);
      if(steer.read().phase==='swipe'){
        d.route='swipe';steer.cancel();
        engine.move(e.clientX,e.clientY);
      }else if(steer.read().phase==='idle')cancel();
    }else engine.move(e.clientX,e.clientY);
  };
  const lost=(e:PointerEvent)=>{if(e.type==='lostpointercapture'&&e.target!==e.currentTarget&&band.current?.hasPointerCapture(e.pointerId))return;if(drag.current?.pointer===e.pointerId)cancel();};
  const onTitleClick=(e:{detail:number})=>{if(e.detail===0||pendingTap.current){pendingTap.current=false;const now=latest.current;(now.onTitle??now.onOverview)?.();}};
  const body = (f: TabFace, role: "prev" | "cur" | "next") =>
    // **What stops this session takes the pill's look** (NOTICE-BRIEF):
    // the kind's mark in the face's seat, the short line in the name's
    // room — each tab its own, so a swipe works the same with one showing.
    renderFace ? (role === "cur" && onTitle ? <button type="button" aria-description={onOverview?gesture:undefined} data-deck-title className="flex h-full min-h-touch min-w-0 flex-1 items-center text-left" onClick={e=>{onTitleClick(e);}}>{renderFace(f,role)}</button> : renderFace(f,role)) : f.interaction?.notices?.length ? (
      role === "cur" && onTitle ? (
        <button
          type="button"
          aria-description={onOverview?gesture:undefined} data-deck-title
          data-deck-notice={f.interaction?.notices[0].kind}
          onClick={(e) => {
            onTitleClick(e);
          }}
          className="flex h-full min-h-touch min-w-0 flex-1 items-center gap-in text-left text-body"
        >
          <NoticeLook list={f.interaction?.notices} seat="size-glass-item-h" />
        </button>
      ) : (
        <span data-deck-notice={f.interaction?.notices[0].kind} className="flex h-full min-w-0 flex-1 items-center gap-in text-body">
          <NoticeLook list={f.interaction?.notices} seat="size-glass-item-h" />
        </span>
      )
    ) : (
    <>
      <span data-deck-face className="grid size-glass-item-h flex-none place-items-center">
        <TabAvatar f={f} bare="size-tab-face" seat="tab" />
      </span>
      {f.rang && <BellMark />}
      {role === "cur" && onTitle ? (
        <button
          type="button"
          aria-description={onOverview?gesture:undefined} data-deck-title
          onClick={(e) => {
            onTitleClick(e);
          }}
          // The finger's floor on the title (the 44 touch rule): the band
          // row is 36 tall, so the button wears it and the row clips it.
          className={cn("flex min-h-touch items-center text-left text-body", f.interaction?.drop && "min-w-0 flex-1")}
        >
          {f.interaction?.drop ? <MidName text={f.interaction?.drop.name} even /> : f.label}
        </button>
      ) : (
        <span aria-description={onOverview?gesture:undefined} data-deck-title={role === "cur" ? "" : undefined} className={cn("text-body", f.interaction?.drop && "flex min-w-0 flex-1")}>
          {f.interaction?.drop ? <MidName text={f.interaction?.drop.name} even /> : f.label}
        </span>
      )}
    </>
    );

  const exploring=steering.phase==='steering'||steering.phase==='pull';
  const position=state.position,movingWay=state.holding?state.direction:Math.sign(position-state.opened);
  const anchor=movingWay<0?Math.ceil(position):Math.floor(position),fraction=Math.abs(position-anchor);
  const slots=nearbySlots(anchor,faces.length,1),past=state.mode==='swipe'&&state.preview!==state.opened;
  const previewing=state.mode==='swipe'&&!state.holding,selecting=state.mode==='swipe';
  return <>
    <div ref={band} data-deck-band data-band-size={faces.length} data-band-past={past||undefined} data-band-phase={exploring?'rail':steering.phase==='pending'?'pending':state.mode}
      data-band-steer-phase={steering.phase} data-band-direction={exploring?steering.direction:state.direction} data-band-preview={faces[exploring?steering.preview:state.preview]?.id}
      data-band-tier={steering.tier} data-band-interval={steering.interval} data-band-pulse={steering.pulse} data-band-overview={steering.overview}
      data-sheet-hold={!!onOverview||exploring||undefined} className="deck-band relative h-full min-w-0 flex-1 overflow-hidden"
      tabIndex={onOverview&&!onTitle?0:undefined} role={onOverview&&!onTitle?'button':undefined} aria-label={onOverview&&!onTitle?gui.switch_tabs(faces.length):undefined}
      aria-description={onOverview&&!onTitle?gesture:undefined}
      style={{touchAction:'none'}} onPointerDown={down} onPointerMove={moving} onPointerUp={e=>finish(e.pointerId)} onPointerCancel={lost} onLostPointerCapture={lost}
      onClick={e=>{if(pendingTap.current||e.detail===0&&!onTitle)onTitleClick(e);}}
      onKeyDown={e=>{if(e.target instanceof Element&&e.target.closest('input,textarea,[contenteditable=true]'))return;
        if(onOverview&&(e.key===(upward?'ArrowUp':'ArrowDown')||!onTitle&&(e.key==='Enter'||e.key===' '))){e.preventDefault();onOverview();}
        else if(faces.length&&(e.key==='ArrowLeft'||e.key==='ArrowRight')){e.preventDefault();focus((at+(e.key==='ArrowRight'?1:-1)+faces.length)%faces.length);}}}
      onContextMenu={e=>{if(state.mode!=='idle'||steering.phase!=='idle')e.preventDefault();}}>
      {faces.length>0&&slots.map(({slot,index})=>{
        const f=faces[index],role=slot===anchor?'cur':slot<anchor?'prev':'next',incoming=slot===anchor+movingWay;
        const share=role==='cur'?1:incoming?fraction:0,offset=role==='cur'?anchor-position:role==='next'?1-fraction:0;
        return <div key={slot} data-band-seg={role} aria-hidden={role!=='cur'||undefined}
          className="deck-band-face absolute inset-0 flex items-center overflow-hidden text-foreground will-change-transform"
          style={{display:share===0?'none':undefined,width:`${share*100}%`,transform:`translate3d(calc(${offset} * var(--deck-band-width)),0,0)`,opacity:previewing&&((past&&index===state.opened)||(!past&&index!==state.opened))?.4:1}}>
          <div data-band-body className="flex h-full flex-none items-center gap-in overflow-hidden whitespace-nowrap pe-row" style={{width:'var(--deck-band-width)'}}>{body(f,f.id===active?'cur':slot<state.opened?'prev':'next')}</div>
        </div>;
      })}
      {selecting&&<span data-band-count className="deck-band-count" aria-hidden="true">{state.preview+1}/{faces.length}</span>}
    </div>
    {exploring&&<DeckExplore placement={placement} band={band} faces={faces} state={steering} origin={drag.current?.x??0} reach={tuning.reach} overview={!!onOverview}/>}
  </>;
}
