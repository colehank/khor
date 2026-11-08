import {useLayoutEffect,useRef,useState} from 'react';
import type {AcpItem,AcpTool} from '@/lib/acp-view';
import type {HistoryRecordRef} from '@/gen/bindings/HistoryRecordRef';
export const recordToolKey=(reference:HistoryRecordRef)=>`${reference.start}:${reference.end}:${reference.item??'record'}`;
type Patch=Partial<AcpTool>&{id:string};
/** Large result records settle the original tool widget. Only small status
 * facts remain when the media/body owner goes offscreen; no second body cache. */
export function useRecordTools(owner:string,references:HistoryRecordRef[]){
 const keys=new Set(references.map(recordToolKey));
 const current=useRef({owner,keys});current.current={owner,keys};
 const [state,setState]=useState<{owner:string;records:Map<string,Patch[]>}>({owner,records:new Map()});
 const offer=(source:string,reference:HistoryRecordRef,items:AcpItem[],full:boolean)=>{
  const key=recordToolKey(reference);
  if(current.current.owner!==source||!current.current.keys.has(key))return;
  const tools=items.filter((item):item is AcpTool&{who:'tool'}=>item.who==='tool').map(item=>full?item:{id:item.id,status:item.status});
  if(!tools.length)return;
  setState(previous=>{
   if(current.current.owner!==source||!current.current.keys.has(key))return previous;
   const records=new Map(previous.owner===source?[...previous.records].filter(([key])=>current.current.keys.has(key)):[]);
   records.set(key,tools);return{owner:source,records};
  });
 };
 const patches=new Map<string,Patch>();
 if(state.owner===owner)for(const reference of references)for(const patch of state.records.get(recordToolKey(reference))??[])patches.set(patch.id,patch);
 useLayoutEffect(()=>{
  setState(previous=>previous.owner===owner?previous:{owner,records:new Map()});
 },[owner]);
 return {offer,patches};
}
