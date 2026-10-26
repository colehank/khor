import {useEffect,useLayoutEffect,useRef,type RefObject} from 'react';

/** Measure against the compact line, then give a wrapping draft the full
 * text row. Synchronous measurement avoids width/height oscillation. */
export function useComposeLayout(ref:RefObject<HTMLDivElement|null>,enabled:boolean){
  const measuring=useRef(false);
  const measure=()=>{
    const root=ref.current,field=root?.querySelector<HTMLTextAreaElement>('[data-composer-text]');if(!root||!field||measuring.current)return;
    if(!enabled){root.removeAttribute('data-lab-compose-expanded');return;}
    measuring.current=true;
    const minHeight=root.style.minHeight;
    // Compact measurement must not enlarge the message viewport and clamp its scrollTop.
    root.style.minHeight=`${root.getBoundingClientRect().height}px`;
    try{
      root.removeAttribute('data-lab-compose-expanded');field.style.height='auto';
      const css=getComputedStyle(field),line=parseFloat(css.lineHeight),padding=parseFloat(css.paddingTop)+parseFloat(css.paddingBottom);
      const expanded=field.value.includes('\n')||field.scrollHeight>line+padding+2;
      root.toggleAttribute('data-lab-compose-expanded',expanded);field.style.height='auto';field.style.height=`${field.scrollHeight}px`;
    }finally{root.style.minHeight=minHeight;measuring.current=false;}
  };
  useLayoutEffect(measure);
  useEffect(()=>{const root=ref.current;if(!enabled||!root)return;const observer=new ResizeObserver(measure);observer.observe(root);root.addEventListener('input',measure);return()=>{observer.disconnect();root.removeEventListener('input',measure);};},[enabled]);
}
