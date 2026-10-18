/** Vertical pans belong to the content sheet until its top detent;
 * remaining travel goes to the content scroller. Text selection keeps its gestures. */
import {sheetGestureReserved} from "@/lib/sheet-gesture";
import {useEffect,useLayoutEffect,useRef} from 'react';
import {onPageWord,pageSheet} from '@/api';
import type {PlaceDetent} from '@/views/SessionPlaceSheet';
import {contentSheetHeights,type ContentSheetBounds} from '@/lib/content-sheet-bounds';

function share(detent: PlaceDetent): number {
  const name = detent === "half" ? "--place-sheet-half" : "--place-sheet-full";
  try {
    const v = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name));
    if (Number.isFinite(v) && v > 0 && v <= 1) return v;
  } catch {
    /* the fallback below */
  }
  return detent === "half" ? 0.5 : 0.92;
}

/** How fast a release has to be moving, in px per ms, to count as a
    throw rather than a placement — the form sheet's own (`ui/dialog`). */
const THROW = 0.5;
const THROW_MS = 80;
/** Under this share of the half detent a release puts the sheet away
    (the prototype's `HALF * 0.55`): a pull that ends a little under half
    springs back, one that has gone most of the way down is a dismissal. */
const AWAY = 0.55;

/**
 * The drag between the detents. Touch, not pointer, events, for the
 * form sheet's reason (`ui/dialog` `useSheet`): a pointer on a scrolling
 * page is taken by the browser's pan before it has moved far, and a touch
 * move that is prevented is the sheet's. **Whose the touch is**: at once
 * on the grabber; elsewhere once it has moved up or down rather than
 * sideways (a row's swipe keeps its own), and only while nothing between
 * the finger and the sheet has scrolled — a list scrolled down scrolls
 * back first. At the full detent a move up is the page's scroll. A
 * slider or anything marked `data-sheet-hold` keeps its touches.
 */
export function useContentSheet(el: HTMLDivElement | null, onDetent: (d: PlaceDetent) => void, onClose: () => void, bounds?: () => ContentSheetBounds, handle?: HTMLElement | null) {
  // Read through a ref: the shell hands a fresh `onOpenChange` on every
  // render (a poll lands every two seconds), and re-binding the listeners
  // mid-drag would drop the pull under the finger.
  const told = useRef({ onDetent, onClose, bounds });
  told.current = { onDetent, onClose, bounds };
  const heights = () => contentSheetHeights(told.current.bounds?.() ?? { height: window.innerHeight, foot: 0 }, {half:share('half'),full:share('full')});
  useLayoutEffect(()=>{
    if(!el)return;
    const sync=()=>{
      const {half,full}=heights();
      el.style.setProperty('--content-sheet-half-height',half+'px');
      el.style.setProperty('--content-sheet-full-height',full+'px');
      // A keyboard transition can shrink the safe range mid-pull.
      if(el.hasAttribute('data-sheet-pulling')&&parseFloat(el.style.height)>full)el.style.height=full+'px';
    };
    sync();
    const observer=new ResizeObserver(sync);
    if(el.parentElement)observer.observe(el.parentElement);
    const top=el.closest('[data-session-screen]')?.querySelector('[data-session-top]');
    if(top)observer.observe(top);
    window.addEventListener('resize',sync);
    window.visualViewport?.addEventListener('resize',sync);
    return()=>{observer.disconnect();window.removeEventListener('resize',sync);window.visualViewport?.removeEventListener('resize',sync);};
  },[el]);
  useEffect(() => {
    if (!el) return;
    const close = () => told.current.onClose();
    const detent = (d: PlaceDetent) => told.current.onDetent(d);
    let from: number | null = null;
    let fromX = 0;
    let pulling = false;
    let vertical = false;
    let live = 0;
    let speed = 0;
    let lastY = 0;
    let lastT = 0;
    let pointer: number | null = null;
    let scroll: HTMLElement | null = null;
    let wheelY=0;
    let wheelEnd: ReturnType<typeof setTimeout> | undefined;
    const scroller=(target:Element|null)=>{for(let n=target;n&&n!==el;n=n.parentElement){if(n instanceof HTMLElement&&/auto|scroll/.test(getComputedStyle(n).overflowY))return n;}return null;};
    const grip = () => {
      pulling = true;
      el.setAttribute("data-sheet-pulling", "");
    };
    const begin = (x: number, y: number, time: number, band: boolean) => {
      el.removeAttribute("data-sheet-dragged");
      from = y;
      fromX = x;
      live = el.getBoundingClientRect().height;
      lastY = from;
      lastT = time;
      speed = 0;
      pulling = false;
      vertical = band;
      if (band) grip();
    };
    const down = (e: TouchEvent) => {
      if (e.touches.length !== 1) return;
      el.removeAttribute("data-sheet-dragged");
      const t = e.target instanceof Element ? e.target : null;
      if (sheetGestureReserved(t)) return;
      const band = !!t?.closest("[data-place-grab]") || !!(t && handle?.contains(t));
      // The content and sheet share vertical pans; selected text and
      // horizontal controls retain their gestures.
      if(!band && window.getSelection()?.isCollapsed===false)return;
      scroll=band?null:scroller(t);
      const control = t?.closest('[data-sheet-hold],input,textarea,[contenteditable=true]');
      // A document editor is also a vertical scrolling surface. Its text
      // is editable, but that alone must not reserve every pan for editing.
      // Real selections and nested input controls still keep their gestures.
      if (!band && control && !control.matches('[data-sheet-scroll]')) return;
      begin(e.touches[0].clientX,e.touches[0].clientY,e.timeStamp,band);
    };
    const drag = (x: number, y: number, time: number, prevent: () => void) => {
      if (from === null) return;
      const dy = y - from;
      if (Math.abs(dy) > 6) el.setAttribute("data-sheet-dragged", "");
      if (!pulling) {
        if (!vertical) {
          const dx = Math.abs(x - fromX);
          if (Math.max(Math.abs(dy), dx) < 6) return;
          if (Math.abs(dy) <= dx) {
            from = null;
            return;
          }
          // Direction is decided once, including while the content scrolls.
          // Reaching its top must not start another slop threshold.
          vertical = true;
        }
        const full=heights().full;
        if ((dy<0&&el.getBoundingClientRect().height>=full-1)||(dy>0&&(scroll?.scrollTop||0)>0)) {
          // Native content scroll owns this portion; rebase at every move
          // so reaching its top can hand off without a jump or a new touch.
          from=y;fromX=x;live=el.getBoundingClientRect().height;lastY=y;lastT=time;
          return;
        }
        grip();
      }
      prevent();
      const {full}=heights(),up=lastY-y;
      if(up>=0){
        const grow=Math.min(up,Math.max(0,full-live));live+=grow;
        if(scroll)scroll.scrollTop+=up-grow;
      }else{
        const back=scroll?Math.min(-up,scroll.scrollTop):0;
        if(scroll)scroll.scrollTop-=back;
        live=Math.max(0,live+up+back);
      }
      el.style.height = `${live}px`;
      const dt = time - lastT;
      if (dt > 0) speed = (y - lastY) / dt;
      lastY = y;
      lastT = time;
    };
    const release = (time: number) => {
      if (from === null) return;
      from = null;
      if (!pulling) return;
      pulling = false;
      el.removeAttribute("data-sheet-pulling");
      el.style.removeProperty("height");
      const {half,full}=heights();
      live=Math.max(0,Math.min(live,full));
      if (time - lastT > THROW_MS) speed = 0;
      // A throw goes one detent its way: up to full, down to half from
      // above it, away from half or under; a placement takes the nearer.
      if (live < half * AWAY || (speed > THROW && live <= half)) return close();
      if (speed < -THROW) return detent("full");
      if (speed > THROW) return detent("half");
      detent(live > (half + full) / 2 ? "full" : "half");
    };
    const cancel = () => { from = null; pulling = false; el.removeAttribute("data-sheet-pulling"); el.style.removeProperty("height"); };
    const move = (e: TouchEvent) => {
      if(e.touches.length!==1)return cancel();
      if(!pulling&&(e.defaultPrevented||window.getSelection()?.isCollapsed===false))return cancel();
      drag(e.touches[0].clientX,e.touches[0].clientY,e.timeStamp,()=>e.preventDefault());
    };
    const up = (e: TouchEvent) => release(e.timeStamp);
    const pointerDown = (e: PointerEvent) => {
      if(e.pointerType==='touch'||e.button!==0)return;
      el.removeAttribute('data-sheet-dragged');
      const target=e.target instanceof Element?e.target:null;
      if(!target?.closest('[data-place-grab]') && !(target&&handle?.contains(target)))return;
      e.preventDefault();begin(e.clientX,e.clientY,e.timeStamp,true);
      pointer=e.pointerId;
    };
    const pointerMove = (e: PointerEvent) => {
      if(e.pointerId!==pointer||from===null)return;
      // Capture only after a real pull, so a stationary press remains a
      // click on the pill's original button rather than on the sheet.
      if(Math.abs(e.clientY-from)>6&&!el.hasPointerCapture(e.pointerId))el.setPointerCapture(e.pointerId);
      drag(e.clientX,e.clientY,e.timeStamp,()=>e.preventDefault());
    };
    const pointerUp = (e: PointerEvent) => {if(e.pointerId!==pointer)return;pointer=null;release(e.timeStamp);};
    const pointerCancel = (e: PointerEvent) => {if(e.pointerId!==pointer)return;pointer=null;cancel();};
    // A pull from the bottom pill is not a click on its child control.
    const click = (e: MouseEvent) => {if(e.detail>0&&el.hasAttribute('data-sheet-dragged')){el.removeAttribute('data-sheet-dragged');e.preventDefault();e.stopImmediatePropagation();}};
    const wheel=(e:WheelEvent)=>{
      if(Math.abs(e.deltaX)>=Math.abs(e.deltaY))return;
      const target=e.target instanceof Element?e.target:null;
      if(sheetGestureReserved(target)||target?.closest('input,textarea'))return;
      const list=scroller(target),{full}=heights();
      const h=el.getBoundingClientRect().height;
      if((e.deltaY<0&&(list?.scrollTop||0)>0)||(e.deltaY>0&&h>=full-1))return;
      if(from===null){scroll=list;wheelY=0;begin(0,0,e.timeStamp,true);}
      wheelY-=e.deltaY;drag(0,wheelY,e.timeStamp,()=>e.preventDefault());
      clearTimeout(wheelEnd);wheelEnd=setTimeout(()=>release(lastT+THROW_MS+1),180);
    };
    let nativeTab:string|null=null,nativeDrop:(()=>void)|null=null,nativeConfig='';
    const native=()=>{
      const tab=el.querySelector<HTMLElement>('[data-native-page-tab]')?.dataset.nativePageTab||null;
      if(tab!==nativeTab){
        nativeDrop?.();if(nativeTab)void pageSheet(nativeTab,null).catch(()=>{});
        nativeTab=tab;nativeConfig='';
        nativeDrop=tab?onPageWord('page:sheet',tab,text=>{
          const [phase,h,v]=text.split(','),height=Number(h),velocity=Number(v);
          if(phase==='cancel'){cancel();return;}
          if(!Number.isFinite(height)||!Number.isFinite(velocity))return;
          const time=performance.now();
          if(phase==='start'){begin(0,0,time,true);}
          else if(!pulling)return;
          live=Math.max(0,Math.min(height,heights().full));speed=velocity/1000;lastT=time;
          el.style.height=live+'px';
          if(phase==='end')release(time);
        }):null;
      }
      if(nativeTab&&!pulling){const config={height:Math.min(el.getBoundingClientRect().height,heights().full),full:heights().full},key=JSON.stringify(config);if(key!==nativeConfig){nativeConfig=key;void pageSheet(nativeTab,config).catch(()=>{});}}
    };
    const nativeResize=new ResizeObserver(native);nativeResize.observe(el);
    const nativeMount=new MutationObserver(native);nativeMount.observe(el,{childList:true,subtree:true});native();
    const suspend=()=>{
      cancel();
      // UIKit normally cancels the pan too; explicitly drop native ownership
      // so a lost lifecycle callback cannot leave a page offset locked.
      if(nativeTab){const tab=nativeTab;nativeConfig='';void pageSheet(tab,null).then(()=>{if(nativeTab===tab)native();}).catch(()=>{});}
    };
    const surfaces = handle ? [el,handle] : [el];
    for(const surface of surfaces){
      surface.addEventListener('touchstart',down,{passive:true});
      surface.addEventListener('touchmove',move,{passive:false});
      surface.addEventListener('touchend',up);
      surface.addEventListener('touchcancel',cancel);
      surface.addEventListener('pointerdown',pointerDown);
      surface.addEventListener('click',click,true);
    }
    window.addEventListener('pointermove',pointerMove);
    window.addEventListener('pointerup',pointerUp);
    window.addEventListener('pointercancel',pointerCancel);
    el.addEventListener('lostpointercapture',pointerCancel);
    el.addEventListener('wheel',wheel,{passive:false});
    window.addEventListener("blur", suspend);
    document.addEventListener("visibilitychange", suspend);
    return () => {
      for(const surface of surfaces){
        surface.removeEventListener('touchstart',down);
        surface.removeEventListener('touchmove',move);
        surface.removeEventListener('touchend',up);
        surface.removeEventListener('touchcancel',cancel);
        surface.removeEventListener('pointerdown',pointerDown);
        surface.removeEventListener('click',click,true);
      }
      window.removeEventListener('pointermove',pointerMove);
      window.removeEventListener('pointerup',pointerUp);
      window.removeEventListener('pointercancel',pointerCancel);
      el.removeEventListener('lostpointercapture',pointerCancel);
      el.removeEventListener('wheel',wheel);clearTimeout(wheelEnd);
      window.removeEventListener("blur", suspend);
      document.removeEventListener("visibilitychange", suspend);
      nativeDrop?.();nativeResize.disconnect();nativeMount.disconnect();if(nativeTab)void pageSheet(nativeTab,null).catch(()=>{});
      cancel();
    };
  }, [el,handle]);
}
