import {useEffect,useRef,type RefObject} from 'react';
import type {HistoryReaderState} from '@/lib/history-reader';
import {canPrefetchOlder} from '@/lib/history-window';
/** Fill short pages and prefetch near the upper edge. Bound each visit to
 * eight pages; repeated cursors and failures require an explicit retry. */
export function useHistoryPrefetch(root:RefObject<HTMLElement|null>,state:HistoryReaderState|null,load:(preserveLatest:boolean)=>void,newer:()=>void,readingLatest:RefObject<boolean>){
 const latest=useRef({load,newer});latest.current={load,newer};
 const visit=useRef({seen:new Set<string>(),count:0});
 useEffect(()=>{
  const element=root.current;if(!element||!state?.loaded||state.busy||state.error)return;
  const ask=(readerUp=false)=>{
   if(element.clientHeight<=0||document.hidden)return;
   const nearTop=element.scrollTop<=element.clientHeight/2;
   const nearBottom=element.scrollHeight-element.clientHeight-element.scrollTop<=element.clientHeight/2;
   if(!nearTop&&!nearBottom){visit.current={seen:new Set(),count:0};return;}
   const older=nearTop&&state.window.older&&canPrefetchOlder(state.window,readingLatest.current,readerUp);
   const forward=!older&&nearBottom&&!state.window.latest;
   const cursor=older?'older:'+older:forward?'newer:'+state.window.newer:null;
   if(!cursor||visit.current.seen.has(cursor)||visit.current.count>=8)return;
   visit.current.seen.add(cursor);visit.current.count++;if(older)latest.current.load(!readerUp&&readingLatest.current);else latest.current.newer();
  };
  const scroll=()=>ask(),wheel=(event:WheelEvent)=>{if(event.deltaY<0)ask(true);};
  let touchY:number|null=null;
  const touchStart=(event:TouchEvent)=>{touchY=event.touches.length===1?event.touches[0].clientY:null;};
  const touchMove=(event:TouchEvent)=>{if(touchY!==null&&event.touches.length===1&&event.touches[0].clientY>touchY)ask(true);};
  const key=(event:KeyboardEvent)=>{if(['ArrowUp','PageUp','Home'].includes(event.key)&&!(event.target instanceof Element&&event.target.closest('input,textarea,[contenteditable=true]')))ask(true);};
  const frame=requestAnimationFrame(()=>ask());
  element.addEventListener('scroll',scroll,{passive:true});element.addEventListener('wheel',wheel,{passive:true});
  element.addEventListener('touchstart',touchStart,{passive:true});element.addEventListener('touchmove',touchMove,{passive:true});element.addEventListener('keydown',key);
  return()=>{cancelAnimationFrame(frame);element.removeEventListener('scroll',scroll);element.removeEventListener('wheel',wheel);element.removeEventListener('touchstart',touchStart);element.removeEventListener('touchmove',touchMove);element.removeEventListener('keydown',key);};
 },[root,state,readingLatest]);
}
