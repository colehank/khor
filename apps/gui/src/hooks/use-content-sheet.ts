/** Existing session content supplies its own host bounds and remembered
 * detent. The pan engine and physical limits are shared with task overlays. */
import {useSheetPan} from '@/hooks/use-sheet-pan';
import {contentSheetHeights,type ContentSheetBounds} from '@/lib/content-sheet-bounds';
import type {SheetDetent} from '@/lib/overlay-policy';
function share(detent: SheetDetent): number {
  const name=detent==='half'?'--place-sheet-half':'--place-sheet-full';
  const value=parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name));
  return Number.isFinite(value)&&value>0&&value<=1?value:detent==='half'?.5:.92;
}
export function useContentSheet(el:HTMLDivElement|null,onDetent:(detent:SheetDetent)=>void,onClose:()=>void,bounds?:()=>ContentSheetBounds,handle?:HTMLElement|null) {
  useSheetPan({element:el,onDetent,onClose,handle,geometry:()=>contentSheetHeights(bounds?.()??{height:window.innerHeight,foot:0},{half:share('half'),full:share('full')})});
}
