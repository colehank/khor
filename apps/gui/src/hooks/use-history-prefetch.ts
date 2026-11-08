import {useEffect,useRef,type RefObject} from 'react';
import type {HistoryReaderState} from '@/lib/history-reader';
/** Fill short pages and prefetch near the upper edge. Bound each visit to
 * eight pages; repeated cursors and failures require an explicit retry. */
export function useHistoryPrefetch(root:RefObject<HTMLElement|null>,state:HistoryReaderState|null,load:()=>void){
 const latest=useRef(load);latest.current=load;
 const visit=useRef({seen:new Set<string>(),count:0});
 useEffect(()=>{
  const element=root.current;if(!element||!state?.loaded||state.busy||state.error||!state.window.older)return;
  const cursor=state.window.older;
  const ask=()=>{
   if(element.clientHeight<=0||document.hidden)return;
   if(element.scrollTop>element.clientHeight/2){visit.current={seen:new Set(),count:0};return;}
   if(visit.current.seen.has(cursor)||visit.current.count>=8)return;
   visit.current.seen.add(cursor);visit.current.count++;latest.current();
  };
  const frame=requestAnimationFrame(ask);
  element.addEventListener('scroll',ask,{passive:true});
  return()=>{cancelAnimationFrame(frame);element.removeEventListener('scroll',ask);};
 },[root,state]);
}
