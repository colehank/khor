/** Right-hand all-window fan. The expanded pill stays production DeckBand. */
import {useEffect,useLayoutEffect,useMemo,useRef,useState,type PointerEvent as ReactPointerEvent} from 'react';
import {createPortal} from 'react-dom';
import {TabAvatar,type TabFace} from '@/browse/TabStrip';
import {IconChevronUp} from '@/components/icons';
import type {SessionHeaderProps} from '@/components/SessionBar';
import {feel} from '@/lib/haptics';
import {readDeckContactLayout,sameDeckContactLayout,type DeckContactLayout} from '@/lib/deck-contact';
import {SETTLE_MS,nearbySlots} from '@/lib/deck-scrub';
import {stillMotion} from '@/lib/travel';
import {useDeckArcPosition,DeckRateCue} from '@/components/DeckExplore';
import {thumbFanJoystick,type FanState,type FanPoint,type FanResult} from './gui-session-fan-state';
import {fanFrame,fanPoint,outerPoint,fanScale,type FanFrame as Frame} from './gui-session-fan-geometry';
import {SessionPill,useSessionPill} from './gui-session-pill';
import './gui-session-fan.css';

type Contact={pointer:number;touch:boolean;touchId:number|null;active:string;ids:string;layout:DeckContactLayout};
const titleOf=(face:TabFace|undefined)=>face?.label??'会话';
function frameFor(button:HTMLElement):Frame{
  const css=getComputedStyle(button),touch=parseFloat(css.getPropertyValue('--touch')),gap=parseFloat(css.getPropertyValue('--gap-row'));
  const b=button.getBoundingClientRect(),cap=button.closest('[data-chat-box]')!.getBoundingClientRect(),screen=button.closest('[data-session-screen]')!,r=screen.getBoundingClientRect(),header=screen.querySelector('[data-lab-model-header]')!.getBoundingClientRect(),v=window.visualViewport;
  const left=Math.max(r.left,v?.offsetLeft??0),top=Math.max(header.bottom,v?.offsetTop??0),width=Math.max(0,Math.min(r.right,(v?.offsetLeft??0)+(v?.width??innerWidth))-left),height=Math.max(0,Math.min(cap.top,(v?.offsetTop??0)+(v?.height??innerHeight))-top);
  return fanFrame({left,top,width,height,touch,gap,originX:b.x+b.width/2-left,originY:b.y+b.height/2-top});
}
type Closing={state:FanState;frame:Frame;contact:Contact;phase:'settling'|'fading';result:FanResult};
function fanItems(position:number,count:number,frame:Frame){
  const reach=frame.slots/2,candidates=nearbySlots(position,count,3).filter(s=>Math.abs(s.slot-position)<reach),unique=new Map<number,(typeof candidates)[number]>();
  for(const item of candidates){const old=unique.get(item.index);if(!old||Math.abs(item.slot-position)<Math.abs(old.slot-position))unique.set(item.index,item);}
  return [...unique.values()].map(({index,slot})=>{
    const offset=slot-position,focus=Math.exp(-offset*offset*.9),edge=Math.min(1,Math.max(0,(reach-Math.abs(offset))/.5));
    return {index,slot,...fanPoint(frame,offset),scale:fanScale(offset),opacity:edge*(.5+.5*focus)};
  });
}
function TitleWords({text}:{text:string}){
  const words=useMemo(()=>Array.from(new Intl.Segmenter('zh',{granularity:'word'}).segment(text),part=>part.segment),[text]);
  return <>{words.map((word,i)=>/\p{Script=Han}/u.test(word)?<span data-fan-word key={i}>{word}</span>:word)}</>;
}
/** A fresh layer owns one continuous cyclic position per contact. */
export function FanLayer({faces,state,frame,phase,duration}:{faces:TabFace[];state:FanState;frame:Frame;phase:string;duration:number}){
  const lastDirection=useRef(0);if(state.direction)lastDirection.current=state.direction;
  const position=useDeckArcPosition(state.hot??Math.max(0,faces.findIndex(f=>f.id===state.ids[0])),Math.max(1,state.ids.length),lastDirection.current,duration);
  const items=fanItems(position,state.ids.length,frame),candidate=state.hot===null?null:faces.find(f=>f.id===state.ids[state.hot!]);
  const extent=frame.placement==='right'&&!frame.horizontal?Math.max(.5,(frame.slots-1)/2):2;
  const steering=state.cursor??(state.deflection??0)*2,neutral=outerPoint(frame,0),cursor=outerPoint(frame,Math.max(-2,Math.min(2,Number.isFinite(steering)?steering:0))*extent/2);
  const guide=Array.from({length:41},(_,i)=>outerPoint(frame,extent*(-1+i/20))).map((p,i)=>`${i?'L':'M'} ${p.x} ${p.y}`).join(' ');
  return createPortal(<div data-lab-left-fan data-fan-placement={frame.placement} data-fan-slots={frame.slots} data-fan-phase={phase} data-fan-lane={state.lane??undefined} data-fan-cancel={state.cancelArmed||undefined} data-fan-selected={candidate?.id} data-fan-position={position} data-fan-total={state.ids.length} data-fan-direction={state.direction} data-fan-tier={state.tier} data-fan-interval={state.interval} data-fan-pulse={state.pulse} data-fan-cursor={steering} data-fan-angle={Math.atan2(state.origin.y-state.point.y,state.origin.x-state.point.x)*180/Math.PI} data-fan-radius={Math.hypot(state.point.x-state.origin.x,state.point.y-state.origin.y)} data-compact={frame.compact||undefined} className="lab-thumb-fan" style={{left:frame.left,top:frame.top,width:frame.width,height:frame.height}}>
    <div className="deck-selector-dimmer"/>
    <div className="lab-fan-identities" data-dimmed={state.lane==='overview'||state.cancelArmed||undefined} aria-hidden="true">
      {items.map(item=>{const id=state.ids[item.index],face=faces.find(f=>f.id===id);return face&&<span key={item.slot} data-lab-fan-session={id} data-selected={state.hot===item.index||undefined} className="lab-fan-item" style={frame.placement==='bottom'?{left:item.x-frame.titleWidth/2,top:item.y-frame.face/2,width:frame.titleWidth,opacity:item.opacity,transform:`scale(${item.scale})`,transformOrigin:`50% ${frame.face/2}px`,gap:frame.gap/3}:{left:item.x-frame.face/2,top:item.y,width:frame.face+frame.gap+frame.titleWidth,opacity:item.opacity,transform:`translateY(-50%) scale(${item.scale})`,transformOrigin:`${frame.face/2}px 50%`,gap:frame.horizontal?frame.gap/1.5:frame.gap}}>
        <span className="lab-fan-face"><TabAvatar f={face} bare="size-list-face" seat="row"/></span>
        <span className="lab-fan-label" data-lab-fan-title style={{width:frame.titleWidth}}><TitleWords text={titleOf(face)}/></span>
      </span>;})}
    </div>
    <div className="lab-fan-steering" aria-hidden="true">
      <svg className="lab-fan-guide" width={frame.width} height={frame.height}><path d={guide}/></svg>
      {[-1,1].map(side=>{const t=side*extent*.6,p=outerPoint(frame,t),q=outerPoint(frame,t+.02),rotation=Math.atan2(q.y-p.y,q.x-p.x)*180/Math.PI;return <span key={side} className="lab-fan-cue" style={{left:p.x,top:p.y,rotate:`${rotation}deg`}}><DeckRateCue side={side} state={{...state,overview:state.armed}} x={0}/></span>;})}
      <span className="deck-selector-origin" data-lab-fan-neutral data-paused={state.direction===0||undefined} style={{left:neutral.x,top:neutral.y}}/>
      <span className="lab-fan-cursor" data-lab-fan-cursor style={{left:cursor.x,top:cursor.y}}/>

    </div>
    <div data-lab-fan-overview data-active={state.lane==='overview'&&state.armed||undefined} className="lab-fan-overview" style={{left:frame.target.x,top:frame.target.y}} aria-hidden="true"><IconChevronUp centred/><span>全部标签</span></div>
    {state.cancelArmed&&<span data-lab-fan-cancel-hint className="lab-fan-cancel-hint" style={{left:Math.max(frame.touch,Math.min(frame.width-frame.touch,frame.originX)),top:Math.max(frame.gap,frame.height-frame.touch*.65)}} aria-hidden="true">松手取消</span>}
  </div>,document.body);
}
export function LabFanNavigator({faces,active,onFocus,onOverview,hidden}:Pick<SessionHeaderProps,'faces'|'active'|'onFocus'|'onOverview'|'hidden'>){
  const root=useRef<HTMLDivElement>(null),button=useRef<HTMLButtonElement>(null),contact=useRef<Contact|null>(null),pendingTap=useRef(false);
  const ids=faces.map(f=>f.id).join('\0'),{expanded,close,toggle}=useSessionPill(root,active,ids,hidden);
  const latest=useRef({faces,active,ids,onFocus,onOverview,close,toggle});latest.current={faces,active,ids,onFocus,onOverview,close,toggle};
  const [state,setState]=useState<FanState|null>(null),[frame,setFrame]=useState<Frame|null>(null),[closing,setClosing]=useState<Closing|null>(null);
  const closingLive=useRef<Closing|null>(null),closingTimer=useRef<ReturnType<typeof setTimeout>|null>(null),frameLive=useRef<Frame|null>(null),timing=useRef({follow:180,fade:120});frameLive.current=frame;
  useLayoutEffect(()=>{if(button.current){const css=getComputedStyle(button.current);timing.current={follow:parseFloat(css.getPropertyValue('--dur-120')),fade:parseFloat(css.getPropertyValue('--dur-120'))};}},[]);
  const previous=useRef(0),cancelCue=useRef(false);
  const engine=useMemo(()=>thumbFanJoystick(next=>{
    if(next.pulse>0&&next.pulse!==previous.current||next.cancelArmed&&!cancelCue.current)feel('threshold');previous.current=next.pulse;cancelCue.current=!!next.cancelArmed;setState(next);
  }),[]);
  const clear=()=>{const c=contact.current;contact.current=null;if(c&&button.current?.hasPointerCapture(c.pointer))button.current.releasePointerCapture(c.pointer);};
  const stopClosing=()=>{if(closingTimer.current)clearTimeout(closingTimer.current);closingTimer.current=null;closingLive.current=null;setClosing(null);};
  const cancel=()=>{pendingTap.current=false;clear();engine.cancel();stopClosing();};
  const focus=(id:string)=>{latest.current.close();latest.current.onFocus(id);},overview=()=>{cancel();latest.current.close();latest.current.onOverview();};
  const finish=(pointer:number,point?:FanPoint)=>{
    const c=contact.current;if(!c||c.pointer!==pointer)return;
    if(!button.current||!sameDeckContactLayout(c.layout,readDeckContactLayout(button.current))||c.active!==latest.current.active||c.ids!==latest.current.ids){cancel();return;}
    if(point)engine.move(point);const snapshot=engine.read(),result=engine.release();clear();
    pendingTap.current=result.kind==='tap';
    if(result.kind==='overview'){overview();return;}
    if(result.kind!=='focus'||!frameLive.current)return;
    if(stillMotion()){focus(result.id);return;}
    const value:Closing={state:snapshot,frame:frameLive.current,contact:c,phase:'settling',result};closingLive.current=value;setClosing(value);
    const valid=()=>!!button.current&&sameDeckContactLayout(c.layout,readDeckContactLayout(button.current))&&latest.current.active===c.active&&latest.current.ids===c.ids;
    closingTimer.current=setTimeout(()=>{
      if(closingLive.current!==value||!valid()){cancel();return;}
      const fading={...value,phase:'fading' as const};closingLive.current=fading;setClosing(fading);
      closingTimer.current=setTimeout(()=>{if(closingLive.current!==fading||!valid()){cancel();return;}stopClosing();if(latest.current.faces.some(f=>f.id===result.id))focus(result.id);},timing.current.fade);
    },SETTLE_MS);
  };
  useLayoutEffect(()=>{cancel();},[engine,active,ids,hidden]);
  useEffect(()=>{
    const key=(e:KeyboardEvent)=>{if(e.key==='Escape'&&(contact.current||closingLive.current)){e.preventDefault();e.stopPropagation();cancel();}},blur=()=>cancel();
    const multi=(e:PointerEvent)=>{if(!e.isPrimary&&(contact.current||closingLive.current)||closingLive.current&&e.target instanceof Node&&!button.current?.contains(e.target))cancel();};
    const start=(e:TouchEvent)=>{
      const c=contact.current;if(!c)return;if(e.touches.length>1){cancel();return;}
      const t=Array.from(e.changedTouches).find(t=>t.target instanceof Node&&button.current?.contains(t.target));if(c.touch&&t)c.touchId=t.identifier;
    };
    const end=(e:TouchEvent)=>{const c=contact.current;if(!c?.touch)return;const t=Array.from(e.changedTouches).find(t=>t.identifier===c.touchId);if(!t)return;if(e.type==='touchcancel')cancel();else finish(c.pointer,{x:t.clientX,y:t.clientY});};
    const move=(e:TouchEvent)=>{if(contact.current&&['fan','overview'].includes(engine.read().phase))e.preventDefault();};
    window.addEventListener('keydown',key,true);window.addEventListener('pointerdown',multi,true);window.addEventListener('blur',blur);window.addEventListener('pagehide',blur);window.addEventListener('resize',blur);window.visualViewport?.addEventListener('resize',blur);document.addEventListener('visibilitychange',blur);
    window.addEventListener('touchstart',start,{capture:true,passive:true});window.addEventListener('touchend',end,{capture:true,passive:true});window.addEventListener('touchcancel',end,{capture:true,passive:true});window.addEventListener('touchmove',move,{capture:true,passive:false});
    return()=>{clear();engine.dispose();if(closingTimer.current)clearTimeout(closingTimer.current);closingLive.current=null;window.removeEventListener('keydown',key,true);window.removeEventListener('pointerdown',multi,true);window.removeEventListener('blur',blur);window.removeEventListener('pagehide',blur);window.removeEventListener('resize',blur);window.visualViewport?.removeEventListener('resize',blur);document.removeEventListener('visibilitychange',blur);window.removeEventListener('touchstart',start,true);window.removeEventListener('touchend',end,true);window.removeEventListener('touchcancel',end,true);window.removeEventListener('touchmove',move,true);};
  },[engine]);
  const down=(e:ReactPointerEvent<HTMLButtonElement>)=>{
    if(!e.isPrimary||e.button!==0||contact.current||!button.current)return;e.preventDefault();stopClosing();pendingTap.current=false;
    const frame=frameFor(button.current);
    setFrame(frame);engine.start({x:e.clientX,y:e.clientY},latest.current.faces.map(f=>f.id),latest.current.active,frame.touch,{x:frame.left+frame.originX,y:frame.top+frame.originY});
    const c:Contact={pointer:e.pointerId,touch:e.pointerType==='touch',touchId:null,active:latest.current.active,ids:latest.current.ids,layout:readDeckContactLayout(button.current)};contact.current=c;button.current.setPointerCapture(c.pointer);
  };
  const shown=closing?.state??state,shownFrame=closing?.frame??frame;
  const visible=!!closing||shown?.phase==='fan'||shown?.phase==='overview',candidate=shown?.hot==null?null:faces.find(f=>f.id===shown.ids[shown.hot!]),current=faces.find(f=>f.id===active);
  const avatarFace=shown?.cancelArmed?current:candidate??current;
  return <div ref={root} className="lab-compose-nav" data-lab-session-navigation data-fan-open={visible||undefined}>
    <button ref={button} type="button" data-lab-compose-avatar data-lab-fan-avatar data-expanded={expanded||undefined} aria-label={`切换会话：${titleOf(current)}`} aria-expanded={expanded||visible} aria-description="点击展开药丸；长按轻推后斜向上下拨动预选，保持偏移循环翻选，弧上回中暂停，继续反拨倒退；回到原头像松手取消；向左上推远后松手展开全部标签，回撤恢复扫选" onPointerDown={down} onPointerMove={e=>{if(contact.current?.pointer===e.pointerId)engine.move({x:e.clientX,y:e.clientY});}} onPointerUp={e=>finish(e.pointerId,{x:e.clientX,y:e.clientY})} onPointerCancel={cancel} onLostPointerCapture={()=>{if(contact.current)cancel();}} onContextMenu={e=>e.preventDefault()} onClick={e=>{if(e.detail===0||pendingTap.current){cancel();toggle();}}} onKeyDown={e=>{if(e.key==='ArrowUp'){e.preventDefault();overview();}}}>
      {avatarFace&&<span key={avatarFace.id} data-lab-fan-avatar-preview={avatarFace.id} className="lab-fan-avatar-face"><TabAvatar f={avatarFace} bare="size-tab-face" seat="tab"/></span>}
    </button>
    {expanded&&<SessionPill faces={faces} active={active} onFocus={focus} onOverview={overview}/>}
    {visible&&shown&&shownFrame&&<FanLayer faces={faces} state={shown} frame={shownFrame} phase={closing?.phase??shown.phase} duration={closing?SETTLE_MS:timing.current.follow}/>}
    <span className="sr-only" role="status" aria-live="polite">{visible?shown?.cancelArmed?'松手取消，离开原头像可恢复预选':shown?.lane==='overview'?shown.armed?'松手展开全部标签':'向左上推远展开，回撤恢复扫选':candidate?`预选 ${titleOf(candidate)}，松手切换，弧上回中暂停，回到原头像松手取消`:'斜向上下轻拨预选，保持偏移循环翻选，回中暂停，向左上推远展开全部标签':''}</span>
  </div>;
}
