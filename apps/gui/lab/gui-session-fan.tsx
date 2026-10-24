/** Right-hand all-window fan. The expanded pill stays production DeckBand. */
import {useEffect,useLayoutEffect,useMemo,useRef,useState,type PointerEvent as ReactPointerEvent} from 'react';
import {createPortal} from 'react-dom';
import {TabAvatar,type TabFace} from '@/browse/TabStrip';
import {IconChevronUp} from '@/components/icons';
import type {SessionHeaderProps} from '@/components/SessionBar';
import {DECK_JOYSTICK_HOLD_MS} from '@/lib/tempo';
import {feel} from '@/lib/haptics';
import {readDeckContactLayout,sameDeckContactLayout,type DeckContactLayout} from '@/lib/deck-contact';
import {SETTLE_MS} from '@/lib/deck-scrub';
import {stillMotion} from '@/lib/travel';
import {useDeckArcPosition} from '@/components/DeckExplore';
import {leftThumbFan,recentCandidates,type FanState,type FanPoint,type FanResult} from './gui-session-fan-state';
import {readLabRecents} from './gui-session-recents';
import {SessionPill,useSessionPill} from './gui-session-pill';
import './gui-session-fan.css';

type Frame={left:number;top:number;width:number;height:number;compact:boolean;anchor:FanPoint;radius:number;touch:number;gap:number;title:{x:number;y:number;width:number};target:{x:number;y:number}};
type Contact={pointer:number;touch:boolean;touchId:number|null;timer:ReturnType<typeof setTimeout>|null;active:string;ids:string;layout:DeckContactLayout};
const titleOf=(face:TabFace|undefined)=>face?.title??face?.label??'会话';
function frameFor(button:HTMLElement,count:number):Frame{
  const css=getComputedStyle(button),touch=parseFloat(css.getPropertyValue('--touch')),gap=parseFloat(css.getPropertyValue('--gap-row'));
  const size=parseFloat(css.fontSize),rawLine=parseFloat(css.getPropertyValue('--text-body--line-height')),line=Number.isFinite(rawLine)?rawLine>4?rawLine:rawLine*size:size*1.5;
  const b=button.getBoundingClientRect(),cap=button.closest('[data-chat-box]')!.getBoundingClientRect(),screen=button.closest('[data-session-screen]')!,r=screen.getBoundingClientRect(),header=screen.querySelector('[data-lab-model-header]')!.getBoundingClientRect(),v=window.visualViewport;
  const left=Math.max(r.left,v?.offsetLeft??0),top=Math.max(header.bottom,v?.offsetTop??0),width=Math.max(0,Math.min(r.right,(v?.offsetLeft??0)+(v?.width??innerWidth))-left),height=Math.max(0,Math.min(cap.top,(v?.offsetTop??0)+(v?.height??innerHeight))-top);
  const x=b.x+b.width/2-left,y=b.y+b.height/2-top,compact=height<4.3*touch+2*line,shown=Math.min(3,count);
  const radius=Math.max(0,Math.min(3.6*touch,(x-touch/2-gap)/Math.cos(12*Math.PI/180)));
  const anchor={x:compact?x-touch-(shown+1)*(touch+gap)/2:x,y:compact?height/2:y-touch*.4};
  const firstX=compact?anchor.x-(shown/2)*(touch+gap):x-radius;
  return {left,top,width,height,compact,anchor,radius,touch,gap,title:{x:gap,y:compact?Math.max(0,height/2-line):Math.max(gap,anchor.y-radius*Math.sin(44*Math.PI/180)-2*line),width:Math.max(touch,Math.min(4.8*touch,firstX-touch/2-2*gap))},target:{x:Math.min(x,width-size*2-gap),y:compact?Math.max(gap,height/2-touch*.6):Math.max(gap,y-2.25*touch)}};
}
type Closing={state:FanState;frame:Frame;contact:Contact;phase:'settling'|'fading';result:FanResult};
function fanItems(position:number,count:number,frame:Frame){
  if(!count)return [];
  const center=count<=3?(count-1)/2:Math.max(1,Math.min(count-2,position)),nearest=Math.round(center);
  return Array.from({length:Math.min(3,count)},(_,i)=>count<=3?i:nearest+i-1).filter(index=>index>=0&&index<count).map(index=>{
    const offset=index-center,distance=index-position,focus=Math.exp(-distance*distance*.9),edge=count<=3?1:Math.min(1,Math.max(0,(1.5-Math.abs(offset))/.5));
    const angle=(44+offset*22)*Math.PI/180;
    return {index,x:frame.compact?frame.anchor.x+offset*(frame.touch+frame.gap):frame.anchor.x-frame.radius*Math.cos(angle),y:frame.compact?frame.anchor.y:frame.anchor.y-frame.radius*Math.sin(angle),scale:.78+.54*focus,opacity:edge*(.4+.6*focus)};
  });
}
export function LabFanNavigator({faces,active,onFocus,onOverview,hidden}:Pick<SessionHeaderProps,'faces'|'active'|'onFocus'|'onOverview'|'hidden'>){
  const root=useRef<HTMLDivElement>(null),button=useRef<HTMLButtonElement>(null),contact=useRef<Contact|null>(null),pendingTap=useRef(false);
  const ids=faces.map(f=>f.id).join('\0'),{expanded,close,toggle}=useSessionPill(root,active,ids,hidden);
  const latest=useRef({faces,active,ids,onFocus,onOverview,close,toggle});latest.current={faces,active,ids,onFocus,onOverview,close,toggle};
  const [touch,setTouch]=useState(44),[state,setState]=useState<FanState|null>(null),[frame,setFrame]=useState<Frame|null>(null),[closing,setClosing]=useState<Closing|null>(null);
  const closingLive=useRef<Closing|null>(null),closingTimer=useRef<ReturnType<typeof setTimeout>|null>(null),frameLive=useRef<Frame|null>(null),timing=useRef({follow:180,fade:120});frameLive.current=frame;
  useLayoutEffect(()=>{if(button.current){const size=parseFloat(getComputedStyle(button.current).getPropertyValue('--touch'));if(Number.isFinite(size))setTouch(size);const css=getComputedStyle(button.current);timing.current={follow:parseFloat(css.getPropertyValue('--dur-180')),fade:parseFloat(css.getPropertyValue('--dur-120'))};}},[]);
  const previous=useRef<string|null>(null);
  const engine=useMemo(()=>leftThumbFan(touch,next=>{
    const target=next.hot===null?null:next.ids[next.hot];
    if(target&&target!==previous.current)feel('threshold');previous.current=target;setState(next);
  }),[touch]);
  const clear=()=>{const c=contact.current;contact.current=null;if(c?.timer)clearTimeout(c.timer);if(c&&button.current?.hasPointerCapture(c.pointer))button.current.releasePointerCapture(c.pointer);};
  const stopClosing=()=>{if(closingTimer.current)clearTimeout(closingTimer.current);closingTimer.current=null;closingLive.current=null;setClosing(null);};
  const cancel=()=>{pendingTap.current=false;clear();engine.cancel();stopClosing();};
  const focus=(id:string)=>{latest.current.close();latest.current.onFocus(id);},overview=()=>{latest.current.close();latest.current.onOverview();};
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
    return()=>{clear();if(closingTimer.current)clearTimeout(closingTimer.current);closingLive.current=null;window.removeEventListener('keydown',key,true);window.removeEventListener('pointerdown',multi,true);window.removeEventListener('blur',blur);window.removeEventListener('pagehide',blur);window.removeEventListener('resize',blur);window.visualViewport?.removeEventListener('resize',blur);document.removeEventListener('visibilitychange',blur);window.removeEventListener('touchstart',start,true);window.removeEventListener('touchend',end,true);window.removeEventListener('touchcancel',end,true);window.removeEventListener('touchmove',move,true);};
  },[engine]);
  const down=(e:ReactPointerEvent<HTMLButtonElement>)=>{
    if(!e.isPrimary||e.button!==0||contact.current||!button.current)return;e.preventDefault();stopClosing();pendingTap.current=false;
    const recent=recentCandidates(readLabRecents(),latest.current.faces.map(f=>f.id),latest.current.active);
    setFrame(frameFor(button.current,recent.length));engine.start({x:e.clientX,y:e.clientY},recent);
    const c:Contact={pointer:e.pointerId,touch:e.pointerType==='touch',touchId:null,timer:null,active:latest.current.active,ids:latest.current.ids,layout:readDeckContactLayout(button.current)};contact.current=c;button.current.setPointerCapture(c.pointer);
    c.timer=setTimeout(()=>{if(contact.current===c)engine.hold();},DECK_JOYSTICK_HOLD_MS);
  };
  const shown=closing?.state??state,shownFrame=closing?.frame??frame;
  const visible=!!closing||shown?.phase==='fan'||shown?.phase==='overview',candidate=shown?.hot==null?null:faces.find(f=>f.id===shown.ids[shown.hot!]),current=faces.find(f=>f.id===active);
  const label=candidate?titleOf(candidate):shown?.ids.length?'向左扫动切换':'只有当前会话';
  const words=useMemo(()=>Array.from(new Intl.Segmenter('zh',{granularity:'word'}).segment(label),part=>part.segment),[label]);
  const position=useDeckArcPosition(closing?closing.state.hot??closing.state.position:shown?.position??0,Math.max(1,shown?.ids.length??1),0,closing?SETTLE_MS:timing.current.follow,!closing,false);
  const items=shown&&shownFrame?fanItems(position,shown.ids.length,shownFrame):[];
  return <div ref={root} className="lab-compose-nav" data-lab-session-navigation data-fan-open={visible||undefined}>
    <button ref={button} type="button" data-lab-compose-avatar data-lab-fan-avatar data-expanded={expanded||undefined} aria-label={`切换会话：${titleOf(current)}`} aria-expanded={expanded||visible} aria-description="点击展开药丸；长按向左扫选所有已打开会话，回到头像取消，松手切换；直接上推展开全部标签" onPointerDown={down} onPointerMove={e=>{if(contact.current?.pointer===e.pointerId)engine.move({x:e.clientX,y:e.clientY});}} onPointerUp={e=>finish(e.pointerId,{x:e.clientX,y:e.clientY})} onPointerCancel={cancel} onLostPointerCapture={()=>{if(contact.current)cancel();}} onContextMenu={e=>e.preventDefault()} onClick={e=>{if(e.detail===0||pendingTap.current){pendingTap.current=false;toggle();}}} onKeyDown={e=>{if(e.key==='ArrowUp'){e.preventDefault();overview();}}}>
      {(candidate??current)&&<span key={(candidate??current)!.id} data-lab-fan-avatar-preview={(candidate??current)!.id} className="lab-fan-avatar-face"><TabAvatar f={(candidate??current)!} bare="size-tab-face" seat="tab"/></span>}
    </button>
    {expanded&&<SessionPill faces={faces} active={active} onFocus={focus} onOverview={overview}/>}
    {visible&&shown&&shownFrame&&createPortal(<div data-lab-left-fan data-fan-phase={closing?.phase??shown.phase} data-fan-lane={shown.lane??undefined} data-fan-selected={candidate?.id} data-fan-position={position} data-fan-total={shown.ids.length} data-compact={shownFrame.compact||undefined} className="lab-thumb-fan" style={{left:shownFrame.left,top:shownFrame.top,width:shownFrame.width,height:shownFrame.height}}>
      <div className="deck-selector-dimmer"/>
      <div className="lab-fan-identities" data-dimmed={shown.lane==='overview'||undefined} aria-hidden="true">
        {items.map(item=>{const id=shown.ids[item.index],face=faces.find(f=>f.id===id);return face&&<span key={id} data-lab-fan-session={id} data-selected={shown.hot===item.index||undefined} className="lab-fan-face" style={{left:item.x,top:item.y,opacity:item.opacity,transform:`translate(-50%,-50%) scale(${item.scale})`}}><TabAvatar f={face} bare="size-list-face" seat="row"/></span>;})}
        <div className="lab-fan-title" style={{left:shownFrame.title.x,top:shownFrame.title.y,width:shownFrame.title.width}}><div className="lab-fan-title-meta"><small>切换会话</small>{shown.hot!==null&&shown.ids.length>3&&<small className="lab-fan-index">{shown.hot+1} / {shown.ids.length}</small>}</div><span key={label} data-lab-fan-title>{words.map((word,i)=>/\p{Script=Han}/u.test(word)?<span data-fan-word key={i}>{word}</span>:word)}</span></div>
      </div>
      <div data-lab-fan-overview data-active={shown.lane==='overview'&&shown.armed||undefined} className="lab-fan-overview" style={{left:shownFrame.target.x,top:shownFrame.target.y}} aria-hidden="true"><IconChevronUp centred/><span>全部标签</span></div>
    </div>,document.body)}
    <span className="sr-only" role="status" aria-live="polite">{visible?shown?.lane==='overview'?shown.armed?'松手展开全部标签':'上推展开，回撤取消':candidate?`预选 ${titleOf(candidate)}，松手切换，回到头像取消`:'向左扫动切换会话，向上展开全部标签':''}</span>
  </div>;
}
