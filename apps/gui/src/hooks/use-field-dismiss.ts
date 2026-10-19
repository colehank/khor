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
    const click=(e:MouseEvent)=>{
      const path=e.composedPath();
      if(latest.current.regions().some(region=>region&&path.some(node=>node instanceof Node&&region.contains(node))))return;
      // A microtask from capture may run before React's click handler.
      // Use the next event-loop task, with no UX delay.
      clearTimeout(pending);
      pending=setTimeout(()=>{if(latest.current.open)latest.current.leave();},0);
    };
    document.addEventListener('click',click,true);
    return()=>{clearTimeout(pending);document.removeEventListener('click',click,true);};
  },[open]);
}
