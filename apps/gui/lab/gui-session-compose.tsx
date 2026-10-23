/** Model-led header and bottom avatar experiment. Everything here is Lab-only. */
import {useEffect,useLayoutEffect,useRef,useState,type RefObject} from 'react';
import {createPortal} from 'react-dom';
import {TabAvatar} from '@/browse/TabStrip';
import {IconBack,IconCompose} from '@/components/icons';
import {TabsGlyph,type SessionHeaderProps} from '@/components/SessionBar';
import {gui} from '@/gen/catalog';
import {DeckBand} from '@/components/DeckBand';
import {DECK_JOYSTICK_DEFAULTS} from '@/lib/deck-joystick';
import './gui-session-compose.css';

/** Compact and expanded states both use the terminal's actual DeckBand.
 * No second pointer recognizer, selector geometry or timing lives here. */
function ComposeNavigator({faces,active,onFocus,onOverview,hidden}:Pick<SessionHeaderProps,'faces'|'active'|'onFocus'|'onOverview'|'hidden'>){
  const root=useRef<HTMLDivElement>(null),[expanded,setExpanded]=useState(false);
  const [reach,setReach]=useState(DECK_JOYSTICK_DEFAULTS.reach);
  useLayoutEffect(()=>{
    const el=root.current;if(!el)return;
    const measure=()=>{
      const b=el.getBoundingClientRect(),v=window.visualViewport,left=v?.offsetLeft??0,right=left+(v?.width??innerWidth),x=b.x+b.width/2;
      const gap=parseFloat(getComputedStyle(el).getPropertyValue('--gap-xxs'));
      // A right-hand seat has less outward travel. Keep the terminal's
      // neutral zone and clocks, fitting only its reach to visible room.
      if(Number.isFinite(gap))setReach(Math.max(DECK_JOYSTICK_DEFAULTS.neutral*2,Math.min(DECK_JOYSTICK_DEFAULTS.reach,x-left-gap,right-x-gap)));
    };
    const observer=new ResizeObserver(measure);observer.observe(el);measure();window.addEventListener('resize',measure);window.visualViewport?.addEventListener('resize',measure);
    return()=>{observer.disconnect();window.removeEventListener('resize',measure);window.visualViewport?.removeEventListener('resize',measure);};
  },[]);
  const close=()=>setExpanded(false),overview=()=>{close();onOverview();},focus=(id:string)=>{close();onFocus(id);};
  const ids=faces.map(f=>f.id).join('\0');
  useEffect(close,[active,ids,hidden]);
  useEffect(()=>{
    if(!expanded)return;
    const outside=(e:globalThis.PointerEvent)=>{if(e.target instanceof Node&&!root.current?.contains(e.target))close();};
    const key=(e:KeyboardEvent)=>{
      if(e.key==='Escape'){e.preventDefault();e.stopPropagation();close();}
      else if(e.target instanceof Element&&e.target.matches('[data-composer-text]'))close();
    };
    const composer=root.current?.closest('[data-chat-box]');composer?.addEventListener('input',close);
    window.addEventListener('pointerdown',outside);window.addEventListener('keydown',key,true);window.addEventListener('resize',close);window.addEventListener('blur',close);window.visualViewport?.addEventListener('resize',close);
    return()=>{composer?.removeEventListener('input',close);window.removeEventListener('pointerdown',outside);window.removeEventListener('keydown',key,true);window.removeEventListener('resize',close);window.removeEventListener('blur',close);window.visualViewport?.removeEventListener('resize',close);};
  },[expanded]);
  return <div ref={root} className="lab-compose-nav" data-lab-session-navigation>
    <div data-lab-compose-avatar data-expanded={expanded||undefined} className="lab-avatar-band">
      <DeckBand faces={faces} active={active} onFocus={focus} onTitle={()=>setExpanded(value=>!value)} onOverview={overview} tuning={{...DECK_JOYSTICK_DEFAULTS,reach}} renderFace={face=><>
        <span className="sr-only">切换会话：{face.title??face.label}，点击{expanded?'收起':'展开'}药丸</span>
        <TabAvatar f={face} bare="size-tab-face" seat="tab"/>
      </>}/>
    </div>
    {expanded&&<div data-lab-session-pill className="glass absolute lab-session-pill" role="group" aria-label="会话药丸">
      <DeckBand faces={faces} active={active} onFocus={focus} onTitle={overview} onOverview={overview}/>
      <button type="button" data-lab-pill-overview className="lab-pill-overview" aria-label="全部标签页" onClick={overview}><TabsGlyph n={faces.length}/></button>
    </div>}
  </div>;
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
