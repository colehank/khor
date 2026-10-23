/** Lab-only identity/navigation. Session configuration is a separate sibling
 * supplied by the existing BarSeat, never part of this button or wheel. */
import {useEffect,useRef,useState,type PointerEvent as ReactPointerEvent} from 'react';
import {createPortal} from 'react-dom';
import {TabAvatar,type TabFace} from '@/browse/TabStrip';
import {IconBack} from '@/components/icons';
import {gui} from '@/gen/catalog';
import type {SessionHeaderProps} from '@/components/SessionBar';
import './gui-session-header.css';

type Slot={face:TabFace;x:number;y:number;angle:number};
type Wheel={x:number;y:number;radius:number;touch:number;slots:Slot[];hot:number|null};
type Contact={id:number;x:number;y:number;held:boolean;timer:ReturnType<typeof setTimeout>|null};
const nameOf=(face:TabFace|undefined)=>face?.title??face?.label??'会话';
const rad=(angle:number)=>angle*Math.PI/180;
function wheelAt(faces:TabFace[],active:string,avatar:HTMLElement):Wheel|null{
  if(!faces.length)return null;
  const css=getComputedStyle(avatar),touch=parseFloat(css.getPropertyValue('--touch'))||44,gap=parseFloat(css.getPropertyValue('--gap-row'))||12;
  const box=avatar.getBoundingClientRect(),x=box.x+box.width/2,y=box.y+box.height/2;
  const radius=Math.min(4*touch,innerWidth-x-touch/2-gap,(visualViewport?.height??innerHeight)-y-touch/2-gap);
  if(radius<touch)return null;
  const at=Math.max(0,faces.findIndex(f=>f.id===active)),count=Math.min(5,faces.length),start=faces.length>5?at-2:0;
  const slots=Array.from({length:count},(_,i)=>{
    const face=faces[(start+i+faces.length)%faces.length],angle=count===1?64:24+i*80/(count-1);
    return {face,angle,x:x+radius*Math.cos(rad(angle)),y:y+radius*Math.sin(rad(angle))};
  });
  return {x,y,radius,touch,slots,hot:null};
}
export function LabSessionHeader({faces,active,onFocus,onBack,onOverview,barRef,children,hidden=false}:SessionHeaderProps){
  const current=faces.find(f=>f.id===active),identity=useRef<HTMLButtonElement>(null),avatar=useRef<HTMLSpanElement>(null);
  const contact=useRef<Contact|null>(null),suppressClick=useRef(false),live=useRef<Wheel|null>(null),latest=useRef({faces,active,onFocus,onOverview});
  latest.current={faces,active,onFocus,onOverview};
  const [wheel,setWheel]=useState<Wheel|null>(null);
  const show=(value:Wheel|null)=>{live.current=value;setWheel(value);};
  const clear=()=>{const c=contact.current;if(c?.timer)clearTimeout(c.timer);contact.current=null;if(c&&identity.current?.hasPointerCapture(c.id))identity.current.releasePointerCapture(c.id);};
  const cancel=()=>{if(contact.current)suppressClick.current=true;clear();show(null);};
  const ids=faces.map(f=>f.id).join('\0');
  useEffect(()=>{cancel();},[active,ids,hidden]);
  useEffect(()=>{
    const escape=(event:KeyboardEvent)=>{if(event.key==='Escape'&&contact.current){event.preventDefault();event.stopPropagation();cancel();}};
    const multi=(event:PointerEvent)=>{if(!event.isPrimary&&contact.current)cancel();};
    const lost=()=>cancel();
    window.addEventListener('keydown',escape,true);window.addEventListener('blur',lost);window.addEventListener('resize',lost);
    window.addEventListener('pointerdown',multi,true);
    visualViewport?.addEventListener('resize',lost);document.addEventListener('visibilitychange',lost);
    return()=>{clear();window.removeEventListener('keydown',escape,true);window.removeEventListener('pointerdown',multi,true);window.removeEventListener('blur',lost);window.removeEventListener('resize',lost);visualViewport?.removeEventListener('resize',lost);document.removeEventListener('visibilitychange',lost);};
  },[]);
  const down=(event:ReactPointerEvent<HTMLButtonElement>)=>{
    if(!event.isPrimary||event.button!==0)return;
    suppressClick.current=false;
    if(contact.current||!(event.target instanceof Element)||!event.target.closest('[data-lab-header-avatar]'))return;
    event.preventDefault();
    const c:Contact={id:event.pointerId,x:event.clientX,y:event.clientY,held:false,timer:null};contact.current=c;
    identity.current?.setPointerCapture(c.id);
    c.timer=setTimeout(()=>{
      if(contact.current!==c||!avatar.current)return;c.held=true;suppressClick.current=true;
      show(wheelAt(latest.current.faces,latest.current.active,avatar.current));
    },350);
  };
  const move=(event:ReactPointerEvent<HTMLButtonElement>)=>{
    const c=contact.current;if(!c||c.id!==event.pointerId)return;
    if(!c.held){if(Math.hypot(event.clientX-c.x,event.clientY-c.y)>10)cancel();return;}
    const w=live.current;if(!w)return;
    const dx=event.clientX-w.x,dy=event.clientY-w.y,distance=Math.hypot(dx,dy),angle=Math.atan2(dy,dx)*180/Math.PI;
    const hot=distance<w.touch||distance>w.radius+w.touch||angle<0||angle>112?null:w.slots.reduce((best,s,i)=>Math.abs(s.angle-angle)<Math.abs(w.slots[best].angle-angle)?i:best,0);
    if(hot!==null&&w.hot!==null&&hot!==w.hot&&Math.abs(w.slots[w.hot].angle-angle)-Math.abs(w.slots[hot].angle-angle)<4)return;
    if(hot!==w.hot)show({...w,hot});
  };
  const up=(event:ReactPointerEvent<HTMLButtonElement>)=>{
    const c=contact.current;if(!c||c.id!==event.pointerId)return;
    const candidate=live.current?.hot==null?null:live.current.slots[live.current.hot]?.face.id,held=c.held;
    clear();show(null);
    if(held&&candidate&&candidate!==latest.current.active&&latest.current.faces.some(f=>f.id===candidate))latest.current.onFocus(candidate);
  };
  const preview=wheel?.hot==null?undefined:wheel.slots[wheel.hot]?.face;
  const first=wheel?.slots[0],last=wheel?.slots.at(-1),arc=wheel&&first&&last?`M ${first.x} ${first.y} A ${wheel.radius} ${wheel.radius} 0 0 1 ${last.x} ${last.y}`:'';
  return <header ref={barRef} data-session-bar data-session-deck data-lab-light-header style={hidden?{visibility:'hidden'}:undefined}>
    <button type="button" data-lab-header-back aria-label={gui.back} onClick={onBack} className="lab-header-icon"><IconBack/></button>
    <button ref={identity} type="button" data-lab-session-identity aria-label={`切换会话：${nameOf(current)}`} aria-description="点击打开会话总览；长按头像沿弧线选择，松手切换，回到头像取消" className="lab-session-identity" onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={cancel} onLostPointerCapture={()=>{if(contact.current)cancel();}} onContextMenu={event=>event.preventDefault()} onClick={event=>{
      if(event.detail!==0&&suppressClick.current){suppressClick.current=false;return;}latest.current.onOverview();
    }}>
      <span ref={avatar} data-lab-header-avatar className="glass"><span aria-hidden>{current&&<TabAvatar f={current} bare="size-tab-face" seat="tab"/>}</span></span>
      <span className="lab-header-words"><span data-lab-header-title>{nameOf(preview??current)}</span>{wheel&&<small>{preview?`${preview.machine?preview.machine+' · ':''}松手切换`:'沿弧线选择 · 回到头像取消'}</small>}</span>
    </button>
    {children}
    {wheel&&createPortal(<div data-lab-session-wheel className="lab-session-wheel" aria-hidden="true">
      <svg className="lab-wheel-track"><path data-rim d={arc}/><path d={arc}/></svg>
      {wheel.slots.map((slot,index)=><div key={slot.face.id} data-lab-wheel-session={slot.face.id} data-selected={wheel.hot===index||undefined} className="lab-wheel-slot" style={{left:slot.x,top:slot.y}}><TabAvatar f={slot.face} bare="size-tab-face" seat="tab"/></div>)}
    </div>,document.body)}
    <span className="sr-only" role="status" aria-live="polite">{wheel?preview?`预览 ${nameOf(preview)}，松手切换`:'沿弧线滑动选择会话，回到头像取消':''}</span>
  </header>;
}
