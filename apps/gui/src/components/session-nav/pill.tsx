import {useEffect,useState,type RefObject} from 'react';
import {DeckBand} from '@/components/DeckBand';
import {TabsGlyph,type SessionHeaderProps} from '@/components/SessionBar';
export function useSessionPill(root:RefObject<HTMLDivElement|null>,active:string,ids:string,hidden:boolean){
  const [expanded,setExpanded]=useState(false),close=()=>setExpanded(false);
  useEffect(close,[active,ids,hidden]);
  useEffect(()=>{
    if(!expanded)return;
    const outside=(e:PointerEvent)=>{if(e.target instanceof Node&&!root.current?.contains(e.target))close();};
    const key=(e:KeyboardEvent)=>{
      if(e.key==='Escape'){e.preventDefault();e.stopPropagation();close();}
      else if(e.target instanceof Element&&e.target.matches('[data-composer-text]'))close();
    };
    const composer=root.current?.closest('[data-session-compose-row]')?.querySelector('[data-chat-box]')??root.current?.closest('[data-chat-box]');composer?.addEventListener('input',close);
    window.addEventListener('pointerdown',outside);window.addEventListener('keydown',key,true);window.addEventListener('resize',close);window.addEventListener('blur',close);window.visualViewport?.addEventListener('resize',close);
    return()=>{composer?.removeEventListener('input',close);window.removeEventListener('pointerdown',outside);window.removeEventListener('keydown',key,true);window.removeEventListener('resize',close);window.removeEventListener('blur',close);window.visualViewport?.removeEventListener('resize',close);};
  },[expanded]);
  return {expanded,close,toggle:()=>setExpanded(value=>!value)};
}
export function SessionPill({faces,active,onFocus,onOverview}:Pick<SessionHeaderProps,'faces'|'active'|'onFocus'|'onOverview'>){
  return <div data-lab-session-pill className="glass absolute lab-session-pill" role="group" aria-label="会话药丸">
    <DeckBand faces={faces} active={active} onFocus={onFocus} onTitle={onOverview} onOverview={onOverview}/>
    <button type="button" data-lab-pill-overview className="lab-pill-overview" aria-label="全部标签页" onClick={onOverview}><TabsGlyph n={faces.length}/></button>
  </div>;
}
