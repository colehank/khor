/** Mouse-only tab order. Preview is local; one identifier-only write lands on
 * release. A drag never focuses, closes or recreates the document it moves. */
import {useEffect,useLayoutEffect,useRef,useState,type PointerEvent as ReactPointerEvent,type MouseEvent as ReactMouseEvent} from 'react';
import {stillMotion} from '@/lib/travel';

type Item = HTMLElement;
type Gesture = {
  id:string;pointer:number;start:number;x:number;grab:number;node:Item;host:HTMLElement;
  started:boolean;order:string[];before:Map<Item,number>;frame:number;turn:number;
};
type Options = {
  host:()=>HTMLElement|null; selector:string; ids:string[];
  onReorder?: (ids:string[])=>void|Promise<unknown>;
  exclude?:string; onStart?:()=>void;
};
function displayedOrder(ids:string[],preview:string[]|null):string[]{
  if(!preview)return ids;
  const wanted=[...new Set(preview)].filter(id=>ids.includes(id)),known=new Set(wanted);let at=0;
  return ids.map(id=>known.has(id)?wanted[at++]:id);
}
export function useTabReorder(options:Options) {
  const latest=useRef(options);latest.current=options;
  const gesture=useRef<Gesture|null>(null),suppress=useRef(false),alive=useRef(true);
  const [preview,setPreview]=useState<string[]|null>(null);
  const shown=useRef(preview),turn=useRef(0);shown.current=preview;
  const idsKey=options.ids.join('\n');
  const rows=(g:Gesture)=>[...g.host.querySelectorAll<Item>(latest.current.selector)];
  const id=(node:Item)=>node.dataset.reorderId!;
  const clearPaint=(g:Gesture)=>{
    cancelAnimationFrame(g.frame);g.node.style.removeProperty('transform');
    delete g.node.dataset.tabDragging;delete g.host.dataset.tabReordering;
    if(g.host.hasPointerCapture(g.pointer))g.host.releasePointerCapture(g.pointer);
  };
  const cancel=()=>{
    const g=gesture.current;if(!g)return;gesture.current=null;clearPaint(g);
    if(g.started)suppress.current=true;
    if(alive.current)setPreview(null);
  };
  useEffect(()=>{
    alive.current=true;
    const abort=()=>cancel();
    const hidden=()=>{if(document.hidden)cancel()};
    // Pointer capture does not imply keyboard focus. Escape belongs to the
    // gesture even if the retained editor/terminal still owns the keyboard.
    const escape=(event:KeyboardEvent)=>{
      if(event.key!=='Escape'||!gesture.current?.started)return;
      event.preventDefault();event.stopPropagation();cancel();
    };
    window.addEventListener('keydown',escape,true);
    window.addEventListener('blur',abort);document.addEventListener('visibilitychange',hidden);
    return()=>{alive.current=false;cancel();window.removeEventListener('keydown',escape,true);window.removeEventListener('blur',abort);document.removeEventListener('visibilitychange',hidden)};
  },[]);
  useEffect(()=>{
    const g=gesture.current;if(!g)return;
    // A live open/close or a layout switch invalidates the measured slots.
    const ids=latest.current.ids;
    if(ids.length!==g.order.length||ids.some(t=>!g.order.includes(t)))cancel();
  },[idsKey]);
  const paint=(g:Gesture)=>{
    g.node.style.transform='';const r=g.node.getBoundingClientRect(),b=g.host.getBoundingClientRect();
    const left=Math.max(b.left,Math.min(b.right-r.width,g.x-g.grab));
    g.node.style.transform=`translateX(${left-r.left}px)`;
  };
  useLayoutEffect(()=>{
    const g=gesture.current;if(!g?.started)return;
    for(const node of rows(g)){
      if(node===g.node)continue;
      const from=g.before.get(node),to=node.getBoundingClientRect().left;
      if(from===undefined||Math.abs(from-to)<1||stillMotion())continue;
      const duration=parseFloat(getComputedStyle(g.host).getPropertyValue('--dur-180'));
      node.animate([{transform:`translateX(${from-to}px)`},{transform:'translateX(0)'}],{duration,easing:'ease-out'});
    }
    g.before.clear();paint(g);
  },[preview]);
  const move=(g:Gesture)=>{
    if(!g.node.isConnected||!g.host.isConnected){cancel();return}
    const list=rows(g),from=g.order.indexOf(g.id);
    // Read target slots without a neighbour's cosmetic travel transform.
    const target=list.find(node=>{
      if(node===g.node||node.dataset.reorderBand!==g.node.dataset.reorderBand)return false;
      const r=node.getBoundingClientRect(),to=g.order.indexOf(id(node));
      return g.x>=r.left&&g.x<=r.right&&((to>from&&g.x>r.left+r.width/2)||(to<from&&g.x<r.left+r.width/2));
    });
    if(target){
      const to=g.order.indexOf(id(target));
      g.before=new Map(list.map(node=>[node,node.getBoundingClientRect().left]));
      for(const node of list)if(node!==g.node)node.getAnimations().forEach(a=>a.cancel());
      const next=g.order.filter(t=>t!==g.id);next.splice(to,0,g.id);g.order=next;setPreview(next);
    }
    paint(g);
  };
  const tick=(g:Gesture)=>{
    if(gesture.current!==g)return;
    const b=g.host.getBoundingClientRect(),edge=parseFloat(getComputedStyle(g.host).getPropertyValue('--gap-pane'));
    const before=g.host.scrollLeft;
    if(g.x<b.left+edge)g.host.scrollLeft-=edge/2;
    else if(g.x>b.right-edge)g.host.scrollLeft+=edge/2;
    if(g.host.scrollLeft!==before)move(g);
    g.frame=requestAnimationFrame(()=>tick(g));
  };
  return {
    order:displayedOrder(options.ids,preview),
    dragging:()=>!!gesture.current?.started,
    handlers:{
      onPointerDownCapture:(e:ReactPointerEvent<HTMLElement>)=>{
        suppress.current=false;
        const o=latest.current,target=e.target as Element,host=o.host();
        if(!o.onReorder||!host||e.pointerType!=='mouse'||e.button!==0||o.ids.length<2)return;
        if(target.closest('[data-tab-close],.session-content-close,input,textarea,[contenteditable=true]')||o.exclude&&target.closest(o.exclude))return;
        const node=target.closest<Item>(o.selector);
        if(!node||!host.contains(node)||!o.ids.includes(id(node)))return;
        gesture.current={id:id(node),pointer:e.pointerId,start:e.clientX,x:e.clientX,grab:e.clientX-node.getBoundingClientRect().left,node,host,started:false,order:displayedOrder(o.ids,shown.current),before:new Map(),frame:0,turn:++turn.current};
      },
      onPointerMoveCapture:(e:ReactPointerEvent<HTMLElement>)=>{
        const g=gesture.current;if(!g||g.pointer!==e.pointerId)return;g.x=e.clientX;
        if(!g.started){
          const threshold=parseFloat(getComputedStyle(g.host).getPropertyValue('--gap-in'));
          if(Math.abs(g.x-g.start)<threshold)return;
          latest.current.onStart?.();g.started=true;g.host.setPointerCapture(g.pointer);
          g.node.dataset.tabDragging='true';g.host.dataset.tabReordering='true';
          g.node.getAnimations().forEach(a=>a.cancel());g.frame=requestAnimationFrame(()=>tick(g));
        }
        e.preventDefault();e.stopPropagation();move(g);
      },
      onPointerUpCapture:(e:ReactPointerEvent<HTMLElement>)=>{
        const g=gesture.current;if(!g||g.pointer!==e.pointerId)return;
        gesture.current=null;clearPaint(g);
        if(!g.started)return;
        e.preventDefault();e.stopPropagation();suppress.current=true;
        const before=latest.current.ids;
        if(before.every((t,i)=>g.order[i]===t)){setPreview(null);return}
        const commit=latest.current.onReorder;
        Promise.resolve().then(()=>commit?.(g.order)).catch(()=>{}).finally(()=>{if(alive.current&&turn.current===g.turn)setPreview(null)});
      },
      onPointerCancel:()=>cancel(),
      onLostPointerCapture:()=>cancel(),
      onContextMenuCapture:()=>cancel(),
      onClickCapture:(e:ReactMouseEvent<HTMLElement>)=>{if(suppress.current){suppress.current=false;e.preventDefault();e.stopPropagation()}},
    },
  };
}
