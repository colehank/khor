/** Right-hand recent-session shortcut. The expanded pill stays production DeckBand. */
import {useEffect,useLayoutEffect,useMemo,useRef,useState,type PointerEvent as ReactPointerEvent} from 'react';
import {createPortal} from 'react-dom';
import {TabAvatar,type TabFace} from '@/browse/TabStrip';
import {IconChevronUp} from '@/components/icons';
import type {SessionHeaderProps} from '@/components/SessionBar';
import {DECK_JOYSTICK_HOLD_MS} from '@/lib/tempo';
import {feel} from '@/lib/haptics';
import {leftThumbFan,recentCandidates,type FanState,type FanPoint} from './gui-session-fan-state';
import {readLabRecents} from './gui-session-recents';
import {SessionPill,useSessionPill} from './gui-session-pill';
import './gui-session-fan.css';

type Frame={left:number;top:number;width:number;height:number;compact:boolean;slots:FanPoint[];title:{x:number;y:number;width:number};target:{x:number;y:number}};
type Contact={pointer:number;touch:boolean;touchId:number|null;timer:ReturnType<typeof setTimeout>|null;active:string;ids:string;layout:string};
const titleOf=(face:TabFace|undefined)=>face?.title??face?.label??'会话';
function layoutStamp(button:HTMLElement){const b=button.getBoundingClientRect(),v=window.visualViewport;return [b.x,b.y,b.width,b.height,innerWidth,innerHeight,v?.width,v?.height,v?.offsetLeft,v?.offsetTop].join(':');}
function frameFor(button:HTMLElement,count:number):Frame{
  const css=getComputedStyle(button),touch=parseFloat(css.getPropertyValue('--touch')),gap=parseFloat(css.getPropertyValue('--gap-row'));
  const size=parseFloat(css.fontSize),rawLine=parseFloat(css.getPropertyValue('--text-body--line-height')),line=Number.isFinite(rawLine)?rawLine>4?rawLine:rawLine*size:size*1.5;
  const b=button.getBoundingClientRect(),cap=button.closest('[data-chat-box]')!.getBoundingClientRect(),screen=button.closest('[data-session-screen]')!,r=screen.getBoundingClientRect(),header=screen.querySelector('[data-lab-model-header]')!.getBoundingClientRect(),v=window.visualViewport;
  const left=Math.max(r.left,v?.offsetLeft??0),top=Math.max(header.bottom,v?.offsetTop??0),width=Math.max(0,Math.min(r.right,(v?.offsetLeft??0)+(v?.width??innerWidth))-left),height=Math.max(0,Math.min(cap.top,(v?.offsetTop??0)+(v?.height??innerHeight))-top);
  const x=b.x+b.width/2-left,y=b.y+b.height/2-top,compact=height<4.3*touch+2*line;
  const radius=Math.max(0,Math.min(3.6*touch,(x-touch/2-gap)/Math.cos(22*Math.PI/180)));
  const slots=Array.from({length:count},(_,i)=>{
    if(compact)return {x:gap+touch/2+i*(touch+gap),y:height/2};
    const angle=(count===1?44:22+i*44/(count-1))*Math.PI/180;
    return {x:x-radius*Math.cos(angle),y:y-radius*Math.sin(angle)};
  });
  const titleX=compact?gap+count*(touch+gap):gap;
  return {left,top,width,height,compact,slots,title:{x:titleX,y:compact?Math.max(0,height/2-line):Math.max(gap,Math.min(...slots.map(s=>s.y),y)-line-gap),width:Math.max(touch,compact?width-titleX-2*touch-2*gap:Math.min(4.4*touch,Math.min(...slots.map(s=>s.x),x)-touch/2-2*gap))},target:{x:Math.min(x,width-size*2-gap),y:compact?Math.max(gap,height/2-touch*.6):Math.max(gap,y-2.25*touch)}};
}
export function LabFanNavigator({faces,active,onFocus,onOverview,hidden}:Pick<SessionHeaderProps,'faces'|'active'|'onFocus'|'onOverview'|'hidden'>){
  const root=useRef<HTMLDivElement>(null),button=useRef<HTMLButtonElement>(null),contact=useRef<Contact|null>(null),pendingTap=useRef(false);
  const ids=faces.map(f=>f.id).join('\0'),{expanded,close,toggle}=useSessionPill(root,active,ids,hidden);
  const latest=useRef({faces,active,ids,onFocus,onOverview,close,toggle});latest.current={faces,active,ids,onFocus,onOverview,close,toggle};
  const [touch,setTouch]=useState(44),[state,setState]=useState<FanState|null>(null),[frame,setFrame]=useState<Frame|null>(null);
  useLayoutEffect(()=>{if(button.current){const size=parseFloat(getComputedStyle(button.current).getPropertyValue('--touch'));if(Number.isFinite(size))setTouch(size);}},[]);
  const previous=useRef<string|null>(null);
  const engine=useMemo(()=>leftThumbFan(touch,next=>{
    const target=next.hot===null?null:next.ids[next.hot];
    if(target&&target!==previous.current)feel('threshold');previous.current=target;setState(next);
  }),[touch]);
  const clear=()=>{const c=contact.current;contact.current=null;if(c?.timer)clearTimeout(c.timer);if(c&&button.current?.hasPointerCapture(c.pointer))button.current.releasePointerCapture(c.pointer);};
  const cancel=()=>{pendingTap.current=false;clear();engine.cancel();};
  const focus=(id:string)=>{latest.current.close();latest.current.onFocus(id);},overview=()=>{latest.current.close();latest.current.onOverview();};
  const finish=(pointer:number,point?:FanPoint)=>{
    const c=contact.current;if(!c||c.pointer!==pointer)return;
    if(!button.current||c.layout!==layoutStamp(button.current)||c.active!==latest.current.active||c.ids!==latest.current.ids){cancel();return;}
    if(point)engine.move(point);const result=engine.release();clear();
    pendingTap.current=result.kind==='tap';
    if(result.kind==='overview')overview();else if(result.kind==='focus'&&latest.current.faces.some(f=>f.id===result.id))focus(result.id);
  };
  useLayoutEffect(()=>{cancel();},[engine,active,ids,hidden]);
  useEffect(()=>{
    const key=(e:KeyboardEvent)=>{if(e.key==='Escape'&&contact.current){e.preventDefault();e.stopPropagation();cancel();}},blur=()=>cancel();
    const multi=(e:PointerEvent)=>{if(!e.isPrimary&&contact.current)cancel();};
    const start=(e:TouchEvent)=>{
      const c=contact.current;if(!c)return;if(e.touches.length>1){cancel();return;}
      const t=Array.from(e.changedTouches).find(t=>t.target instanceof Node&&button.current?.contains(t.target));if(c.touch&&t)c.touchId=t.identifier;
    };
    const end=(e:TouchEvent)=>{const c=contact.current;if(!c?.touch)return;const t=Array.from(e.changedTouches).find(t=>t.identifier===c.touchId);if(!t)return;if(e.type==='touchcancel')cancel();else finish(c.pointer,{x:t.clientX,y:t.clientY});};
    const move=(e:TouchEvent)=>{if(contact.current&&['fan','overview'].includes(engine.read().phase))e.preventDefault();};
    window.addEventListener('keydown',key,true);window.addEventListener('pointerdown',multi,true);window.addEventListener('blur',blur);window.addEventListener('pagehide',blur);window.addEventListener('resize',blur);window.visualViewport?.addEventListener('resize',blur);document.addEventListener('visibilitychange',blur);
    window.addEventListener('touchstart',start,{capture:true,passive:true});window.addEventListener('touchend',end,{capture:true,passive:true});window.addEventListener('touchcancel',end,{capture:true,passive:true});window.addEventListener('touchmove',move,{capture:true,passive:false});
    return()=>{clear();window.removeEventListener('keydown',key,true);window.removeEventListener('pointerdown',multi,true);window.removeEventListener('blur',blur);window.removeEventListener('pagehide',blur);window.removeEventListener('resize',blur);window.visualViewport?.removeEventListener('resize',blur);document.removeEventListener('visibilitychange',blur);window.removeEventListener('touchstart',start,true);window.removeEventListener('touchend',end,true);window.removeEventListener('touchcancel',end,true);window.removeEventListener('touchmove',move,true);};
  },[engine]);
  const down=(e:ReactPointerEvent<HTMLButtonElement>)=>{
    if(!e.isPrimary||e.button!==0||contact.current||!button.current)return;e.preventDefault();pendingTap.current=false;
    const recent=recentCandidates(readLabRecents(),latest.current.faces.map(f=>f.id),latest.current.active);
    setFrame(frameFor(button.current,recent.length));engine.start({x:e.clientX,y:e.clientY},recent);
    const c:Contact={pointer:e.pointerId,touch:e.pointerType==='touch',touchId:null,timer:null,active:latest.current.active,ids:latest.current.ids,layout:layoutStamp(button.current)};contact.current=c;button.current.setPointerCapture(c.pointer);
    c.timer=setTimeout(()=>{if(contact.current===c)engine.hold();},DECK_JOYSTICK_HOLD_MS);
  };
  const visible=state?.phase==='fan'||state?.phase==='overview',candidate=state?.hot==null?null:faces.find(f=>f.id===state.ids[state.hot!]),current=faces.find(f=>f.id===active);
  const label=candidate?titleOf(candidate):state?.ids.length?'向左扫选':'暂无最近会话';
  const words=useMemo(()=>Array.from(new Intl.Segmenter('zh',{granularity:'word'}).segment(label),part=>part.segment),[label]);
  return <div ref={root} className="lab-compose-nav" data-lab-session-navigation data-fan-open={visible||undefined}>
    <button ref={button} type="button" data-lab-compose-avatar data-lab-fan-avatar data-expanded={expanded||undefined} aria-label={`切换会话：${titleOf(current)}`} aria-expanded={expanded||visible} aria-description="点击展开药丸；长按向左扫选最近三个会话，回到头像取消，松手切换；直接上推展开全部标签" onPointerDown={down} onPointerMove={e=>{if(contact.current?.pointer===e.pointerId)engine.move({x:e.clientX,y:e.clientY});}} onPointerUp={e=>finish(e.pointerId,{x:e.clientX,y:e.clientY})} onPointerCancel={cancel} onLostPointerCapture={()=>{if(contact.current)cancel();}} onContextMenu={e=>e.preventDefault()} onClick={e=>{if(e.detail===0||pendingTap.current){pendingTap.current=false;toggle();}}} onKeyDown={e=>{if(e.key==='ArrowUp'){e.preventDefault();overview();}}}>
      {current&&<TabAvatar f={current} bare="size-tab-face" seat="tab"/>}
    </button>
    {expanded&&<SessionPill faces={faces} active={active} onFocus={focus} onOverview={overview}/>}
    {visible&&state&&frame&&createPortal(<div data-lab-left-fan data-fan-phase={state.phase} data-fan-lane={state.lane??undefined} data-fan-selected={candidate?.id} data-compact={frame.compact||undefined} className="lab-thumb-fan" style={{left:frame.left,top:frame.top,width:frame.width,height:frame.height}}>
      <div className="deck-selector-dimmer"/>
      <div className="lab-fan-identities" data-dimmed={state.lane==='overview'||undefined} aria-hidden="true">
        {state.ids.map((id,i)=>{const face=faces.find(f=>f.id===id),at=frame.slots[i];return face&&at&&<span key={id} data-lab-fan-session={id} data-selected={state.hot===i||undefined} className="lab-fan-face" style={{left:at.x,top:at.y}}><TabAvatar f={face} bare="size-list-face" seat="row"/></span>;})}
        <div className="lab-fan-title" style={{left:frame.title.x,top:frame.title.y,width:frame.title.width}}><small>最近会话</small><span data-lab-fan-title>{words.map((word,i)=>/\p{Script=Han}/u.test(word)?<span data-fan-word key={i}>{word}</span>:word)}</span></div>
      </div>
      <div data-lab-fan-overview data-active={state.lane==='overview'&&state.armed||undefined} className="lab-fan-overview" style={{left:frame.target.x,top:frame.target.y}} aria-hidden="true"><IconChevronUp centred/><span>全部标签</span></div>
    </div>,document.body)}
    <span className="sr-only" role="status" aria-live="polite">{visible?state?.lane==='overview'?state.armed?'松手展开全部标签':'上推展开，回撤取消':candidate?`预选 ${titleOf(candidate)}，松手切换，回到头像取消`:'向左扫选最近会话，向上展开全部标签':''}</span>
  </div>;
}
