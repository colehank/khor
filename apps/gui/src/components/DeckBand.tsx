/** Shared mobile pill gesture: direct manipulation or long-press rate control. */
import {useEffect,useLayoutEffect,useMemo,useRef,useState,type ReactNode,type PointerEvent} from 'react';
import {createPortal} from 'react-dom';
import {MidName,TabAvatar,type TabFace} from '@/browse/TabStrip';
import {BellMark} from '@/components/BellMark';
import {NoticeLook} from '@/components/PillNotice';
import {cn} from '@/lib/utils';
import {feel} from '@/lib/haptics';
import {stillMotion} from '@/lib/travel';
import {deckScrubber,holdTiming,nearbySlots,type DeckScrubState} from '@/lib/deck-scrub';
import './deck-band.css';

type Drag={pointer:number;touch:boolean;touchId:number|null;title:boolean;rail:boolean};
type Geometry={left:number;top:number;right:number;step:number};
export function DeckBand({faces,active,onFocus,onTitle,renderFace}: {
  faces:TabFace[];active:string;onFocus:(id:string)=>void;onTitle?:()=>void;
  renderFace?:(face:TabFace,role:'prev'|'cur'|'next')=>ReactNode;
}) {
  const band=useRef<HTMLDivElement>(null),drag=useRef<Drag|null>(null);
  const latest=useRef({faces,active,onFocus,onTitle});
  const at=Math.max(0,faces.findIndex(f=>f.id===active));
  const [state,setState]=useState<DeckScrubState>({opened:at,preview:at,position:at,mode:'idle',source:'swipe',direction:0,holding:false,...holdTiming(0,250)});
  const [geometry,setGeometry]=useState<Geometry>({left:0,top:0,right:0,step:92});
  const alive=useRef(true),prior=useRef(at),travelDirection=useRef(0);
  const engine=useMemo(()=>deckScrubber(faces.length,at,s=>{
    if(!alive.current)return;
    if(s.preview!==prior.current&&(s.mode==='swipe'||s.mode==='rail'))feel('threshold');
    prior.current=s.preview;
    if(s.holding)travelDirection.current=s.direction;
    else if(s.mode==='idle'||s.mode==='pending')travelDirection.current=0;
    setState(s);
    if(s.mode==='rail'&&drag.current){
      drag.current.rail=true;
      if(!band.current?.hasPointerCapture(drag.current.pointer))band.current?.setPointerCapture(drag.current.pointer);
    }
  },index=>{
    const now=latest.current;
    if(now.faces[index]&&now.faces[index].id!==now.active)now.onFocus(now.faces[index].id);
  }),[faces.length]);
  const cancel=()=>{
    const d=drag.current;drag.current=null;engine.cancel();
    if(d&&band.current?.hasPointerCapture(d.pointer))band.current.releasePointerCapture(d.pointer);
  };
  useLayoutEffect(()=>{latest.current={faces,active,onFocus,onTitle};});
  const ids=JSON.stringify(faces.map(f=>f.id));
  useLayoutEffect(()=>{cancel();engine.select(at);},[engine,active,ids]);
  useEffect(()=>{
    alive.current=true;
    return()=>{alive.current=false;engine.cancel();};
  },[engine]);
  useEffect(()=>{
    const b=band.current;if(!b)return;
    let width=b.clientWidth;
    const measure=()=>b.style.setProperty('--deck-band-width',`${b.clientWidth}px`);measure();
    const resize=new ResizeObserver(()=>{if(width!==b.clientWidth){if(drag.current)cancel();width=b.clientWidth;measure();}});resize.observe(b);
    const blur=()=>cancel(),hidden=()=>{if(document.visibilityState!=='visible')cancel();};
    const key=(e:KeyboardEvent)=>{if(e.key==='Escape'&&drag.current){cancel();e.preventDefault();e.stopPropagation();}};
    const start=(e:TouchEvent)=>{
      const d=drag.current;
      if(e.touches.length>1){cancel();return;}
      if(!d?.touch||d.touchId!==null)return;
      const t=Array.from(e.changedTouches).find(t=>t.target instanceof Node&&b.contains(t.target));
      if(t)d.touchId=t.identifier;
    };
    const end=(e:TouchEvent)=>{
      const d=drag.current;if(!d?.touch)return;
      const t=Array.from(e.changedTouches).find(t=>t.identifier===d.touchId);if(!t)return;
      if(e.type==='touchcancel')cancel();else finish(d.pointer);
    };
    // Long press owns this contact until release, including outside the pill.
    // Initial vertical pans still belong to the sheet.
    const move=(e:TouchEvent)=>{if(drag.current?.rail)e.preventDefault();};
    window.addEventListener('touchstart',start,{capture:true,passive:true});
    window.addEventListener('touchend',end,{capture:true,passive:true});
    window.addEventListener('touchcancel',end,{capture:true,passive:true});
    window.addEventListener('touchmove',move,{passive:false,capture:true});
    window.addEventListener('blur',blur);window.addEventListener('pagehide',blur);
    document.addEventListener('visibilitychange',hidden);window.addEventListener('keydown',key,true);
    return()=>{resize.disconnect();window.removeEventListener('touchstart',start,true);window.removeEventListener('touchend',end,true);window.removeEventListener('touchcancel',end,true);window.removeEventListener('touchmove',move,true);window.removeEventListener('blur',blur);window.removeEventListener('pagehide',blur);document.removeEventListener('visibilitychange',hidden);window.removeEventListener('keydown',key,true);};
  },[engine]);
  const finish=(id:number)=>{
    const d=drag.current;if(!d||id!==d.pointer)return;
    drag.current=null;
    const result=engine.release(stillMotion());
    if(band.current?.hasPointerCapture(id))band.current.releasePointerCapture(id);
    if(result==='tap'&&d.title)latest.current.onTitle?.();
  };
  const down=(e:PointerEvent)=>{
    if(!e.isPrimary){cancel();return;}
    if(e.button!==0||drag.current||!band.current)return;
    if(engine.read().mode==='settling'){engine.arrive();return;}
    if(e.target instanceof Element&&e.target.closest('input,textarea,[contenteditable=true],[data-band-control]'))return;
    if(window.getSelection()?.isCollapsed===false)return;
    const b=band.current,r=b.getBoundingClientRect(),cs=getComputedStyle(b);
    const deck=b.closest('[data-session-deck], [data-content-deck], .mobile-content-deck')
      ??b.closest('[data-file-pill]')?.parentElement;
    const bounds=deck?.getBoundingClientRect()??r;
    const step=parseFloat(cs.getPropertyValue('--glass-btn'))+2*parseFloat(cs.getPropertyValue('--gap-pane'));
    setGeometry({left:Math.max(0,bounds.left),right:Math.max(0,window.innerWidth-bounds.right),top:r.top,step:Number.isFinite(step)?step:92});
    drag.current={pointer:e.pointerId,touch:e.pointerType==='touch',touchId:null,title:!!(e.target instanceof Element&&e.target.closest('[data-band-seg="cur"]')),rail:false};
    engine.start(e.clientX,e.clientY,b.clientWidth,Number.isFinite(step)?step:92);
    if(!renderFace)b.setPointerCapture(e.pointerId);
  };
  const moving=(e:PointerEvent)=>{
    const d=drag.current;if(!d||d.pointer!==e.pointerId)return;
    if(d.rail)e.preventDefault();
    engine.move(e.clientX,e.clientY);
    const mode=engine.read().mode;
    if(mode==='idle'){if(!d.rail)cancel();return;}
    if((mode==='swipe'||mode==='rail')&&!band.current?.hasPointerCapture(e.pointerId))band.current?.setPointerCapture(e.pointerId);
  };
  const lost=(e:PointerEvent)=>{
    if(e.type==='lostpointercapture'&&e.target!==e.currentTarget&&band.current?.hasPointerCapture(e.pointerId))return;
    if(drag.current?.pointer===e.pointerId)cancel();
  };
  const body = (f: TabFace, role: "prev" | "cur" | "next") =>
    // **What stops this session takes the pill's look** (NOTICE-BRIEF):
    // the kind's mark in the face's seat, the short line in the name's
    // room — each tab its own, so a swipe works the same with one showing.
    renderFace ? (role === "cur" && onTitle ? <button type="button" data-deck-title className="flex h-full min-h-touch min-w-0 flex-1 items-center text-left" onClick={e=>{if(e.detail===0)onTitle();}}>{renderFace(f,role)}</button> : renderFace(f,role)) : f.interaction?.notices?.length ? (
      role === "cur" && onTitle ? (
        <button
          type="button"
          data-deck-title
          data-deck-notice={f.interaction?.notices[0].kind}
          onClick={(e) => {
            if (e.detail === 0) onTitle();
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
          data-deck-title
          onClick={(e) => {
            if (e.detail === 0) onTitle();
          }}
          // The finger's floor on the title (the 44 touch rule): the band
          // row is 36 tall, so the button wears it and the row clips it.
          className={cn("flex min-h-touch items-center text-left text-body", f.interaction?.drop && "min-w-0 flex-1")}
        >
          {f.interaction?.drop ? <MidName text={f.interaction?.drop.name} even /> : f.label}
        </button>
      ) : (
        <span data-deck-title={role === "cur" ? "" : undefined} className={cn("text-body", f.interaction?.drop && "flex min-w-0 flex-1")}>
          {f.interaction?.drop ? <MidName text={f.interaction?.drop.name} even /> : f.label}
        </span>
      )}
    </>
    );

  const rail=state.mode==='rail'||state.mode==='settling'&&state.source==='rail';
  const position=rail?at:state.position;
  const movingWay=state.holding?state.direction
    :state.mode==='settling'&&travelDirection.current?travelDirection.current
    :Math.sign(position-state.opened);
  const anchor=movingWay<0?Math.ceil(position):Math.floor(position);
  const fraction=Math.abs(position-anchor);
  const slots=nearbySlots(anchor,faces.length,1);
  const past=state.mode==='swipe'&&state.preview!==state.opened;
  const previewing=state.mode==='swipe'&&!state.holding;
  const selecting=state.mode==='swipe';
  return <>
    <div ref={band} data-deck-band data-band-past={past||undefined} data-band-phase={state.mode} data-band-preview={faces[state.preview]?.id}
      data-sheet-hold={rail||undefined} className="deck-band relative h-full min-w-0 flex-1 overflow-hidden"
      style={{touchAction:'none'}} onPointerDown={down} onPointerMove={moving}
      onPointerUp={e=>finish(e.pointerId)} onPointerCancel={lost} onLostPointerCapture={lost}
      onContextMenu={e=>{if(state.mode!=='idle')e.preventDefault();}}>
      {faces.length>0&&slots.map(({slot,index})=>{
        const f=faces[index],role=slot===anchor?'cur':slot<anchor?'prev':'next';
        const incoming=slot===anchor+movingWay;
        const share=role==='cur'?1:incoming?fraction:0;
        const offset=role==='cur'?anchor-position:role==='next'?1-fraction:0;
        return <div key={slot} data-band-seg={role} aria-hidden={role!=='cur'||undefined}
          className="deck-band-face absolute inset-0 flex items-center overflow-hidden text-foreground will-change-transform"
          style={{display:share===0?'none':undefined,width:`${share*100}%`,transform:`translate3d(calc(${offset} * var(--deck-band-width)),0,0)`,opacity:previewing&&((past&&index===state.opened)||(!past&&index!==state.opened))?.4:1}}>
          <div data-band-body className="flex h-full flex-none items-center gap-in overflow-hidden whitespace-nowrap pe-row" style={{width:'var(--deck-band-width)'}}>
            {body(f,f.id===active?'cur':slot<state.opened?'prev':'next')}
          </div>
        </div>;
      })}
      {selecting&&<span data-band-count className="deck-band-count" aria-hidden="true">{state.preview+1}/{faces.length}</span>}
    </div>
    {rail&&faces.length>0&&createPortal(<div data-deck-selector data-sheet-hold role="listbox" aria-label={faces[at]?.label}
      className="glass fixed deck-selector" data-leaving={state.mode==='settling'||undefined}
      style={{left:geometry.left,right:geometry.right,bottom:window.innerHeight-geometry.top}}>
      <div className="deck-selector-count" aria-hidden="true">{state.preview+1} / {faces.length}</div>
      <div className="deck-selector-window">
        <span className="deck-selector-focus"/>
        {nearbySlots(state.position,faces.length,Math.min(3,Math.floor(faces.length/2))).map(({slot,index})=><div key={slot} role="option"
          aria-selected={slot===Math.round(state.position)} className="deck-selector-item"
          style={{width:geometry.step,transform:`translate3d(${(slot-state.position)*geometry.step}px,0,0)`,opacity:Math.max(.4,1-Math.abs(slot-state.position)*.25)}}>
          {!faces[index].hideFace&&<TabAvatar f={faces[index]} bare="size-list-face" seat="row"/>}<span>{faces[index].label}</span>
        </div>)}
      </div>
    </div>,document.body)}
  </>;
}
