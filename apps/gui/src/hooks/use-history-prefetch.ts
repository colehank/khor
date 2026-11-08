import {useEffect,useRef,type RefObject} from 'react';
import type {HistoryReaderState} from '@/lib/history-reader';
/** Fill short pages and prefetch near the upper edge. Bound each visit to
 * eight pages; repeated cursors and failures require an explicit retry. */
export function useHistoryPrefetch(root:RefObject<HTMLElement|null>,state:HistoryReaderState|null,load:()=>void,newer:()=>void){
 const latest=useRef({load,newer});latest.current={load,newer};
 const visit=useRef({seen:new Set<string>(),count:0});
 useEffect(()=>{
  const element=root.current;if(!element||!state?.loaded||state.busy||state.error)return;
  const ask=()=>{
   if(element.clientHeight<=0||document.hidden)return;
   const nearTop=element.scrollTop<=element.clientHeight/2;
   const nearBottom=element.scrollHeight-element.clientHeight-element.scrollTop<=element.clientHeight/2;
   if(!nearTop&&!nearBottom){visit.current={seen:new Set(),count:0};return;}
   const older=nearTop&&state.window.older;
   const forward=!older&&nearBottom&&!state.window.latest;
   const cursor=older?'older:'+older:forward?'newer:'+state.window.newer:null;
   if(!cursor||visit.current.seen.has(cursor)||visit.current.count>=8)return;
   visit.current.seen.add(cursor);visit.current.count++;if(older)latest.current.load();else latest.current.newer();
  };
  const frame=requestAnimationFrame(ask);
  element.addEventListener('scroll',ask,{passive:true});
  return()=>{cancelAnimationFrame(frame);element.removeEventListener('scroll',ask);};
 },[root,state]);
}
