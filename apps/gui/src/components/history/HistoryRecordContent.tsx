import {useEffect,useLayoutEffect,useRef,useState} from 'react';
import type {HistoryRecordRef} from '@/gen/bindings/HistoryRecordRef';
import type {AcpItem} from '@/lib/acp-view';
import type {HistoryItem} from '@/lib/history-paint';
import {historyApi} from '@/lib/history-api';
import {admitHistoryRecord,retainHistoryRecord} from '@/lib/history-record-budget';
import {presentHistoryRecord} from '@/lib/history-record-presentation';
import {AgentContent} from '@/components/ai-elements/content';
import {Markdown} from '@/components/Markdown';
import {Button} from '@/components/ui/button';
import {HistoryRecords} from './HistoryRecords';
import type {HistoryExportControl} from '@/hooks/use-history-export';
import {gui} from '@/gen/catalog';

/** The source, decoded media and in-flight work belong to this visible
 * record, not a second transcript cache. Offscreen records release all three. */
export function HistoryRecordContent({id,binding,reference,exporter,onOpenFile,owner,existingTools,onTools,requested}:{
 id:string;binding:string;reference:HistoryRecordRef;exporter:HistoryExportControl;onOpenFile:(path:string)=>void;
 requested:boolean;owner:string;existingTools:ReadonlySet<string>;onTools:(owner:string,reference:HistoryRecordRef,items:AcpItem[],full:boolean)=>void;
}){
 const host=useRef<HTMLDivElement>(null),[visible,setVisible]=useState(false),[attempt,setAttempt]=useState(0);
 const [items,setItems]=useState<HistoryItem[]|null>(null),[error,setError]=useState('');
 const active=visible||requested;
 const tools=useRef(onTools);tools.current=onTools;
 const [reservedHeight,setReservedHeight]=useState(0);
 const insertion=useRef<{scroll:HTMLElement;height:number}|null>(null);
 useLayoutEffect(()=>{const before=insertion.current;insertion.current=null;if(before&&host.current)before.scroll.scrollTop+=host.current.getBoundingClientRect().height-before.height;},[items]);
 const [expanded,setExpanded]=useState<ReadonlySet<string>>(new Set());
 useEffect(()=>{
  if(!host.current)return;
  const observer=new IntersectionObserver(entries=>{const next=entries.some(entry=>entry.isIntersecting);if(!next&&host.current)setReservedHeight(host.current.getBoundingClientRect().height);setVisible(next);},{rootMargin:'100% 0%'});
  observer.observe(host.current);return()=>observer.disconnect();
 },[]);
 useEffect(()=>{
  setItems(null);setError('');if(!active)return;
  const stop=new AbortController(),source=historyApi(id,binding);
  let release: (()=>void)|null=null;let settled:AcpItem[]=[];
  void (async()=>{const leave=await admitHistoryRecord(stop.signal);try{
    const value=await presentHistoryRecord(source,reference,stop.signal);
    if(!stop.signal.aborted){release=retainHistoryRecord(value);settled=value.filter((item):item is HistoryItem&AcpItem=>item.who!=='sys');tools.current(owner,reference,settled,true);const rect=host.current?.getBoundingClientRect(),scroll=host.current?.closest<HTMLElement>('[data-chat-scroll]');if(rect&&scroll&&rect.top<scroll.getBoundingClientRect().top)insertion.current={scroll,height:rect.height};setItems(value);}
  }finally{leave();}})().catch(failure=>{
   if(!stop.signal.aborted)setError(failure instanceof Error?failure.message:String(failure));
  }).finally(()=>{void source.dispose();});
  return()=>{stop.abort();release?.();tools.current(owner,reference,settled,false);void source.dispose();};
 },[id,binding,reference.cursor,reference.start,reference.end,reference.item,active,attempt,owner]);
 return <div ref={host} data-history-record-content style={{minHeight:visible&&items?undefined:reservedHeight||undefined}} className="min-h-touch py-in">
  {visible&&!items&&!error&&<p role="status" className="text-caption text-muted-foreground">正在读取内容</p>}
  {error&&<p role="status" className="text-caption text-muted-foreground">{error} <Button size="sm" variant="ghost" onClick={()=>setAttempt(value=>value+1)}>{gui.region_retry}</Button></p>}
  {items?.filter(item=>item.who!=='tool'||!existingTools.has(item.id)).map(item=>item.who==='sys'?<p key={item.historyKey} className="text-caption">{item.text}</p>:item.who==='user'?
   <div key={item.historyKey} role="article" aria-label="你" className="py-row"><Markdown text={item.text}/></div>:
   <AgentContent key={item.historyKey} item={item} streaming={false} active={false} onOpenFile={onOpenFile}
    open={item.who==='tool'&&expanded.has(item.id)} onOpenChange={shown=>{if(item.who==='tool')setExpanded(now=>{const next=new Set(now);if(shown)next.add(item.id);else next.delete(item.id);return next;});}}/>)}
  <details className="text-caption text-muted-foreground"><summary>原始记录</summary>
   <HistoryRecords references={[reference]} exporter={exporter}
    read={async(cursor,from,signal)=>{const source=historyApi(id,binding);try{return await source.record(cursor,from,signal);}finally{await source.dispose();}}}/>
  </details>
 </div>;
}
