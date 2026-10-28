/** A field and its portaled candidates are one interaction region.
 * Decide outside-ness at capture, but dismiss after the click has run:
 * inline candidates must not move the pressed action before activation. */
import {useEffect,useRef} from 'react';
export function useFieldDismiss(open:boolean, regions:()=>Array<HTMLElement|null>, leave:()=>void) {
  const latest=useRef({open,regions,leave});
  latest.current={open,regions,leave};
  useEffect(()=>{
    if(!open)return;
    let pending: ReturnType<typeof setTimeout> | undefined;
    const inside=(e:Event)=>latest.current.regions().some(region=>region&&e.composedPath().some(node=>node instanceof Node&&region.contains(node)));
    let pressed:{id:number;inside:boolean}|null=null;
    const down=(e:PointerEvent)=>{
      if(e.button===0&&e.isPrimary!==false)pressed={id:e.pointerId,inside:inside(e)};
    };
    const cancel=(e:PointerEvent)=>{if(pressed?.id===e.pointerId)pressed=null;};
    const click=(e:MouseEvent)=>{
      if(e.button!==0)return;
      const owned=e.detail>0&&pressed?.inside===true;
      pressed=null;
      if(owned||inside(e))return;
      // A microtask from capture may run before React's click handler.
      // Use the next event-loop task, with no UX delay.
      clearTimeout(pending);
      pending=setTimeout(()=>{if(latest.current.open)latest.current.leave();},0);
    };
    document.addEventListener('pointerdown',down,true);
    document.addEventListener('pointercancel',cancel,true);
    document.addEventListener('click',click,true);
    return()=>{clearTimeout(pending);document.removeEventListener('pointerdown',down,true);document.removeEventListener('pointercancel',cancel,true);document.removeEventListener('click',click,true);};
  },[open]);
}
