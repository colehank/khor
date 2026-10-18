/** Task sheets use the content sheet's pan engine. Only their geometry and
 * default stop differ; menus, text selection, scroll handoff and cancellation
 * keep one implementation. */
import {useCallback,useLayoutEffect,useState} from 'react';
import {useSheetPan} from '@/hooks/use-sheet-pan';
import {overlayHeights,type overlayPolicy} from '@/lib/overlay-policy';
type Policy=ReturnType<typeof overlayPolicy>;

function contentHeight(el: HTMLElement): number {
  const css=getComputedStyle(el);
  const px=(value:string)=>parseFloat(value)||0;
  const children=[...el.children].filter((child):child is HTMLElement=>child instanceof HTMLElement&&getComputedStyle(child).display!=='none'&&!['absolute','fixed'].includes(getComputedStyle(child).position));
  return px(css.paddingTop)+px(css.paddingBottom)+px(css.borderTopWidth)+px(css.borderBottomWidth)+Math.max(0,children.length-1)*px(css.rowGap)+children.reduce((height,child)=>{
    const style=getComputedStyle(child);
    const intrinsic=child.dataset.slot==='dialog-body'?contentHeight(child):child.offsetHeight+Math.max(0,child.scrollHeight-child.clientHeight);
    return height+intrinsic+px(style.marginTop)+px(style.marginBottom);
  },0);
}
export function useTaskOverlay(el: HTMLDivElement|null, policy: Policy, dismiss: ()=>void) {
  const sheet=policy.presentation==='sheet'?el:null;
  const [detent,setDetent]=useState<'half'|'full'>(()=>policy.initial==='large'?'full':'half');
  const [resizable,setResizable]=useState(false);
  useLayoutEffect(()=>{setDetent(policy.initial==='large'?'full':'half');},[sheet,policy.initial]);
  useLayoutEffect(()=>{
    if(!sheet || !policy.expandOnInput)return;
    const expand=(event:Event)=>{if(event.target instanceof Element && event.target.matches('input,textarea,[contenteditable="true"]'))setDetent('full');};
    sheet.addEventListener('focusin',expand);
    if(sheet.contains(document.activeElement) && document.activeElement?.matches('input,textarea,[contenteditable="true"]'))setDetent('full');
    return()=>sheet.removeEventListener('focusin',expand);
  },[sheet,policy.expandOnInput]);
  const geometry=useCallback(()=>{
    if(!sheet)return {half:0,full:0,room:0};
    const root=getComputedStyle(document.documentElement);
    const top=(parseFloat(root.getPropertyValue('--safe-top'))||0)+(parseFloat(root.getPropertyValue('--gap-pane'))||0);
    const bounds={height:window.visualViewport?.height??window.innerHeight,top};
    const heights=overlayHeights(bounds,{initial:policy.initial,maximum:policy.maximum,content:contentHeight(sheet)});
    const room=overlayHeights(bounds,{initial:'large',maximum:policy.maximum==='fit'?'large':policy.maximum,content:0}).full;
    return {...heights,room};
  },[sheet,policy.initial,policy.maximum]);
  useSheetPan({element:sheet,onDetent:setDetent,onClose:dismiss,geometry,dismissible:policy.dismissible});
  useLayoutEffect(()=>{
    if(!el)return;
    if(!sheet){
      setResizable(false);
      el.style.removeProperty('--task-sheet-height');
      el.removeAttribute('data-detent');
      return;
    }
    const sync=()=>{
      const {half,full,room}=geometry();
      sheet.style.setProperty("--task-sheet-room",`${room}px`);
      setResizable(was=>was===(full-half>1)?was:full-half>1);
      sheet.style.setProperty('--task-sheet-height',`${detent==='full'?full:half}px`);
    };
    const resize=new ResizeObserver(sync);
    const observe=()=>{
      resize.disconnect();resize.observe(sheet);
      for(const child of sheet.children)if(child instanceof HTMLElement){resize.observe(child);if(child.dataset.slot==='dialog-body')for(const row of child.children)resize.observe(row);}
      sync();
    };
    observe();
    const content=new MutationObserver(observe);
    content.observe(sheet,{childList:true,subtree:true,characterData:true});
    window.addEventListener('resize',sync);window.visualViewport?.addEventListener('resize',sync);
    return()=>{resize.disconnect();content.disconnect();window.removeEventListener('resize',sync);window.visualViewport?.removeEventListener('resize',sync);};
  },[el,sheet,detent,geometry]);
  return {detent,resizable,cycle:()=>setDetent(was=>was==='half'?'full':'half')};
}
