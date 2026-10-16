/** Vertical pans belong to the content sheet until its top detent;
 * remaining travel goes to the content scroller. Text selection keeps its gestures. */
import {useEffect,useRef} from 'react';
import type {PlaceDetent} from '@/views/SessionPlaceSheet';

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
export function useContentSheet(el: HTMLDivElement | null, onDetent: (d: PlaceDetent) => void, onClose: () => void, bounds?: () => { height: number; foot: number }, handle?: HTMLElement | null) {
  // Read through a ref: the shell hands a fresh `onOpenChange` on every
  // render (a poll lands every two seconds), and re-binding the listeners
  // mid-drag would drop the pull under the finger.
  const told = useRef({ onDetent, onClose, bounds });
  told.current = { onDetent, onClose, bounds };
  useEffect(() => {
    if (!el) return;
    const close = () => told.current.onClose();
    const detent = (d: PlaceDetent) => told.current.onDetent(d);
    let from: number | null = null;
    let fromX = 0;
    let fromH = 0;
    let pulling = false;
    let live = 0;
    let speed = 0;
    let lastY = 0;
    let lastT = 0;
    let pointer: number | null = null;
    let scroll: HTMLElement | null = null;
    let wheelY=0;
    let wheelEnd: ReturnType<typeof setTimeout> | undefined;
    const scroller=(target:Element|null)=>{for(let n=target;n&&n!==el;n=n.parentElement){if(n instanceof HTMLElement&&/auto|scroll/.test(getComputedStyle(n).overflowY))return n;}return null;};
    const scrolled = (t: Element | null) => {
      for (let n = t; n && n !== el; n = n.parentElement) {
        if (n instanceof HTMLElement && n.scrollTop > 0) return true;
      }
      return false;
    };
    const grip = () => {
      pulling = true;
      el.setAttribute("data-sheet-pulling", "");
    };
    const begin = (x: number, y: number, time: number, band: boolean) => {
      el.removeAttribute("data-sheet-dragged");
      from = y;
      fromX = x;
      fromH = el.getBoundingClientRect().height;
      live = fromH;
      lastY = from;
      lastT = time;
      speed = 0;
      pulling = false;
      if (band) grip();
    };
    const down = (e: TouchEvent) => {
      if (e.touches.length !== 1) return;
      el.removeAttribute("data-sheet-dragged");
      const t = e.target instanceof Element ? e.target : null;
      const band = !!t?.closest("[data-place-grab]") || !!(t && handle?.contains(t));
      // The content and sheet share vertical pans; selected text and
      // horizontal controls retain their gestures.
      if(!band && window.getSelection()?.isCollapsed===false)return;
      scroll=band?null:scroller(t);
      if (!band && (t?.closest('[data-sheet-hold], input[type="range"]') || scrolled(t))) return;
      begin(e.touches[0].clientX,e.touches[0].clientY,e.timeStamp,band);
    };
    const drag = (x: number, y: number, time: number, prevent: () => void) => {
      if (from === null) return;
      const dy = y - from;
      if (Math.abs(dy) > 6) el.setAttribute("data-sheet-dragged", "");
      if (!pulling) {
        const dx = Math.abs(x - fromX);
        if (Math.max(Math.abs(dy), dx) < 6) return;
        if (Math.abs(dy) <= dx || (dy < 0 && el.getAttribute("data-detent") === "full")) {
          from = null;
          return;
        }
        grip();
      }
      prevent();
      const box = told.current.bounds?.() ?? { height: window.innerHeight, foot: 0 };
      const wanted=fromH-dy,full=share("full")*box.height-box.foot;
      live = Math.max(0, Math.min(wanted, full));
      if(scroll)scroll.scrollTop=Math.max(0,wanted-full);
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
      const box = told.current.bounds?.() ?? { height: window.innerHeight, foot: 0 };
      const half = share("half") * box.height - box.foot;
      const full = share("full") * box.height - box.foot;
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
      if(target?.closest('input,textarea,[data-sheet-hold]'))return;
      const list=scroller(target),box=told.current.bounds?.()??{height:innerHeight,foot:0};
      const h=el.getBoundingClientRect().height,full=share('full')*box.height-box.foot;
      if((list?.scrollTop||0)>0||(e.deltaY>0&&h>=full-1))return;
      if(from===null){scroll=null;wheelY=0;begin(0,0,e.timeStamp,true);}
      wheelY-=e.deltaY;drag(0,wheelY,e.timeStamp,()=>e.preventDefault());
      clearTimeout(wheelEnd);wheelEnd=setTimeout(()=>release(lastT+THROW_MS+1),180);
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
    window.addEventListener("blur", cancel);
    document.addEventListener("visibilitychange", cancel);
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
      window.removeEventListener("blur", cancel);
      document.removeEventListener("visibilitychange", cancel);
      cancel();
    };
  }, [el,handle]);
}
