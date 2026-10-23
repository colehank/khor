/** Model-led header and bottom avatar experiment. Everything here is Lab-only. */
import {useEffect,useLayoutEffect,useRef,useState,type RefObject,type PointerEvent as ReactPointerEvent} from 'react';
import {createPortal} from 'react-dom';
import {TabAvatar,type TabFace} from '@/browse/TabStrip';
import {IconBack,IconCompose} from '@/components/icons';
import {TabsGlyph,type SessionHeaderProps} from '@/components/SessionBar';
import {gui} from '@/gen/catalog';
import './gui-session-compose.css';

type Slot={face:TabFace;x:number;y:number;width:number};
type Wheel={x:number;y:number;touch:number;slots:Slot[];hot:number|null;all:boolean;target:{x:number;y:number}};
type Contact={id:number;x:number;y:number;held:boolean;timer:ReturnType<typeof setTimeout>|null};
const titleOf=(f:TabFace|undefined)=>f?.title??f?.label??'会话';
function outerWheel(faces:TabFace[],active:string,button:HTMLElement):Wheel|null{
  if(!faces.length)return null;
  const css=getComputedStyle(button),touch=parseFloat(css.getPropertyValue('--touch')),gap=parseFloat(css.getPropertyValue('--gap-in'));
  if(!Number.isFinite(touch)||!Number.isFinite(gap))return null;
  const b=button.getBoundingClientRect(),x=b.x+b.width/2,y=b.y+b.height/2;
  const header=button.closest('[data-session-screen]')?.querySelector('[data-lab-model-header]')?.getBoundingClientRect().bottom??0;
  const minimum=header+touch/2+gap,first=y-touch-2*gap,step=touch+gap;
  const count=Math.min(5,faces.length,Math.max(1,Math.floor((first-minimum)/step)+1));
  const at=Math.max(0,faces.findIndex(f=>f.id===active)),start=faces.length>count?at-Math.floor(count/2):0;
  const target={x:innerWidth-touch-gap,y:Math.max(minimum,y-3*touch)};
  const slots=Array.from({length:count},(_,i)=>{
    const t=count===1?0:i/(count-1),sx=x+touch*1.1*(1-Math.sqrt(1-t*t));
    return {face:faces[(start+i+faces.length)%faces.length],x:sx,y:Math.max(minimum,first-i*step),width:Math.max(touch,target.x-touch-sx+touch/2)};
  });
  return {x,y,touch,slots,hot:null,all:false,target};
}
function ComposeNavigator({faces,active,onFocus,onOverview,hidden}:Pick<SessionHeaderProps,'faces'|'active'|'onFocus'|'onOverview'|'hidden'>){
  const current=faces.find(f=>f.id===active),button=useRef<HTMLButtonElement>(null),contact=useRef<Contact|null>(null),suppress=useRef(false);
  const live=useRef<Wheel|null>(null),latest=useRef({faces,active,onFocus,onOverview});latest.current={faces,active,onFocus,onOverview};
  const [wheel,setWheel]=useState<Wheel|null>(null);
  const show=(w:Wheel|null)=>{live.current=w;setWheel(w);};
  const clear=()=>{const c=contact.current;contact.current=null;if(c?.timer)clearTimeout(c.timer);if(c&&button.current?.hasPointerCapture(c.id))button.current.releasePointerCapture(c.id);};
  const cancel=()=>{if(contact.current)suppress.current=true;clear();show(null);};
  const ids=faces.map(f=>f.id).join('\0');
  useEffect(()=>{cancel();},[active,ids,hidden]);
  useEffect(()=>{
    const key=(e:KeyboardEvent)=>{if(e.key==='Escape'&&contact.current){e.preventDefault();e.stopPropagation();cancel();}};
    const multi=(e:PointerEvent)=>{if(!e.isPrimary&&contact.current)cancel();},lost=()=>cancel();
    window.addEventListener('keydown',key,true);window.addEventListener('pointerdown',multi,true);window.addEventListener('blur',lost);window.addEventListener('resize',lost);visualViewport?.addEventListener('resize',lost);document.addEventListener('visibilitychange',lost);
    return()=>{clear();window.removeEventListener('keydown',key,true);window.removeEventListener('pointerdown',multi,true);window.removeEventListener('blur',lost);window.removeEventListener('resize',lost);visualViewport?.removeEventListener('resize',lost);document.removeEventListener('visibilitychange',lost);};
  },[]);
  const down=(e:ReactPointerEvent<HTMLButtonElement>)=>{
    if(!e.isPrimary||e.button!==0)return;suppress.current=false;if(contact.current)return;e.preventDefault();
    const c:Contact={id:e.pointerId,x:e.clientX,y:e.clientY,held:false,timer:null};contact.current=c;button.current?.setPointerCapture(c.id);
    c.timer=setTimeout(()=>{if(contact.current!==c||!button.current)return;c.held=true;suppress.current=true;show(outerWheel(latest.current.faces,latest.current.active,button.current));},350);
  };
  const move=(e:ReactPointerEvent<HTMLButtonElement>)=>{
    const c=contact.current;if(!c||c.id!==e.pointerId)return;
    if(!c.held){if(Math.hypot(e.clientX-c.x,e.clientY-c.y)>10)cancel();return;}
    const w=live.current;if(!w)return;
    const neutral=Math.hypot(e.clientX-w.x,e.clientY-w.y)<w.touch;
    const all=!neutral&&Math.hypot(e.clientX-w.target.x,e.clientY-w.target.y)<w.touch;
    let hot:number|null=null;
    if(!neutral&&!all){
      const distances=w.slots.map(s=>Math.hypot(e.clientX-s.x,e.clientY-s.y)),best=distances.reduce((a,d,i)=>d<distances[a]?i:a,0);
      if(distances[best]<w.touch*1.25)hot=best;
      if(hot!==null&&w.hot!==null&&hot!==w.hot&&distances[w.hot]-distances[hot]<w.touch/8)hot=w.hot;
    }
    if(hot!==w.hot||all!==w.all)show({...w,hot,all});
  };
  const up=(e:ReactPointerEvent<HTMLButtonElement>)=>{
    const c=contact.current;if(!c||c.id!==e.pointerId)return;
    const w=live.current,id=w?.hot==null?null:w.slots[w.hot]?.face.id,held=c.held,all=w?.all;clear();show(null);
    if(held&&all)latest.current.onOverview();else if(held&&id&&id!==latest.current.active&&latest.current.faces.some(f=>f.id===id))latest.current.onFocus(id);
  };
  return <>
    <button ref={button} type="button" data-lab-compose-avatar title={`切换会话：${titleOf(current)}`} aria-label={`切换会话：${titleOf(current)}`} aria-description="点击打开总览，长按沿外弧选择；斜上推到全部标签页，松手打开列表" onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={cancel} onLostPointerCapture={()=>{if(contact.current)cancel();}} onContextMenu={e=>e.preventDefault()} onClick={e=>{if(e.detail!==0&&suppress.current){suppress.current=false;return;}latest.current.onOverview();}}>
      {current&&<TabAvatar f={current} bare="size-tab-face" seat="tab"/>}
    </button>
    {wheel&&createPortal(<div data-lab-outer-wheel aria-hidden="true" className="lab-outer-wheel">
      <div data-lab-wheel-dim className="lab-outer-scrim"/>
      <div className="lab-outer-origin" style={{left:wheel.x,top:wheel.y}}>{current&&<TabAvatar f={current} bare="size-tab-face" seat="tab"/>}</div>
      {wheel.slots.map((s,i)=><div data-lab-outer-session={s.face.id} data-selected={wheel.hot===i||undefined} key={s.face.id} className="lab-outer-row" style={{left:s.x-wheel.touch/2,top:s.y-wheel.touch/2,width:s.width}}>
        <span className="lab-outer-face"><TabAvatar f={s.face} bare="size-tab-face" seat="tab"/></span><span className="lab-outer-words"><span>{titleOf(s.face)}</span><small>{s.face.machine??s.face.session?.category??''}{s.face.id===active?' · 当前':''}</small></span>
      </div>)}
      <div data-lab-all-tabs-target data-selected={wheel.all||undefined} className="lab-outer-all" style={{left:wheel.target.x,top:wheel.target.y}}><TabsGlyph n={faces.length}/><span>全部标签页</span></div>
    </div>,document.body)}
    <span role="status" aria-live="polite" className="sr-only">{wheel?wheel.all?'松手打开全部标签页':wheel.hot===null?'沿外弧滑动选择会话，回到头像取消':`预览 ${titleOf(wheel.slots[wheel.hot].face)}，松手切换`:''}</span>
  </>;
}
export function LabModelHeader(props:SessionHeaderProps){
  const {barRef,onBack,onNew,newLabel,children,hidden}=props,[slot,setSlot]=useState<Element|null>(null);
  useLayoutEffect(()=>{
    const root=barRef.current?.closest('[data-session-screen]');if(!root)return;
    const find=()=>setSlot(root.querySelector('[data-lab-composer-session-slot]'));find();
    const observer=new MutationObserver(find);observer.observe(root,{childList:true,subtree:true});return()=>observer.disconnect();
  },[barRef,props.active]);
  return <>
    <header ref={barRef} data-session-bar data-session-deck data-lab-model-header style={hidden?{visibility:'hidden'}:undefined}>
      <button type="button" className="lab-header-icon" data-lab-model-back aria-label={gui.back} onClick={onBack}><IconBack/></button>
      <div data-lab-model-settings>{children}</div>
      {onNew&&<button type="button" className="glass lab-compose-new" data-lab-new-session aria-label={newLabel??'新建会话'} onClick={onNew}><IconCompose/></button>}
    </header>
    {slot&&createPortal(<ComposeNavigator {...props}/>,slot)}
  </>;
}

/** Measure against the compact line, then give a wrapping draft the full
 * text row. Synchronous measurement avoids width/height oscillation. */
export function useLabComposeLayout(ref:RefObject<HTMLDivElement|null>,enabled:boolean){
  const measuring=useRef(false);
  const measure=()=>{
    const root=ref.current,field=root?.querySelector<HTMLTextAreaElement>('[data-composer-text]');if(!root||!field||measuring.current)return;
    if(!enabled){root.removeAttribute('data-lab-compose-expanded');return;}
    measuring.current=true;root.removeAttribute('data-lab-compose-expanded');field.style.height='auto';
    const css=getComputedStyle(field),line=parseFloat(css.lineHeight),padding=parseFloat(css.paddingTop)+parseFloat(css.paddingBottom);
    const expanded=field.value.includes('\n')||field.scrollHeight>line+padding+2;
    root.toggleAttribute('data-lab-compose-expanded',expanded);field.style.height='auto';field.style.height=`${field.scrollHeight}px`;measuring.current=false;
  };
  useLayoutEffect(measure);
  useEffect(()=>{const root=ref.current;if(!enabled||!root)return;const observer=new ResizeObserver(measure);observer.observe(root);root.addEventListener('input',measure);return()=>{observer.disconnect();root.removeEventListener('input',measure);};},[enabled]);
}
