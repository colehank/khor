import {useEffect,useRef,useState} from 'react';
import type {HistoryRecordRef} from '@/gen/bindings/HistoryRecordRef';
import type {HistoryItem} from '@/lib/history-paint';
import {historyApi} from '@/lib/history-api';
import {presentHistoryRecord} from '@/lib/history-record-presentation';
import {AgentContent} from '@/components/ai-elements/content';
import {Markdown} from '@/components/Markdown';
import {Button} from '@/components/ui/button';
import {HistoryRecords} from './HistoryRecords';
import type {HistoryExportControl} from '@/hooks/use-history-export';
import {gui} from '@/gen/catalog';

/** The source, decoded media and in-flight work belong to this visible
 * record, not a second transcript cache. Offscreen records release all three. */
export function HistoryRecordContent({id,binding,reference,exporter,onOpenFile}:{
 id:string;binding:string;reference:HistoryRecordRef;exporter:HistoryExportControl;onOpenFile:(path:string)=>void;
}){
 const host=useRef<HTMLDivElement>(null),[visible,setVisible]=useState(false),[attempt,setAttempt]=useState(0);
 const [items,setItems]=useState<HistoryItem[]|null>(null),[error,setError]=useState('');
 const [expanded,setExpanded]=useState<ReadonlySet<string>>(new Set());
 useEffect(()=>{
  if(!host.current)return;
  const observer=new IntersectionObserver(entries=>setVisible(entries.some(entry=>entry.isIntersecting)),{rootMargin:'100% 0px'});
  observer.observe(host.current);return()=>observer.disconnect();
 },[]);
 useEffect(()=>{
  setItems(null);setError('');if(!visible)return;
  const stop=new AbortController(),source=historyApi(id,binding);
  void presentHistoryRecord(source,reference,stop.signal).then(value=>{if(!stop.signal.aborted)setItems(value);},failure=>{
   if(!stop.signal.aborted)setError(failure instanceof Error?failure.message:String(failure));
  }).finally(()=>{void source.dispose();});
  return()=>{stop.abort();void source.dispose();};
 },[id,binding,reference.cursor,reference.start,reference.end,reference.item,visible,attempt]);
 return <div ref={host} data-history-record-content className="min-h-touch py-in">
  {!items&&!error&&<p role="status" className="text-caption text-muted-foreground">正在读取内容</p>}
  {error&&<p role="status" className="text-caption text-muted-foreground">{error} <Button size="sm" variant="ghost" onClick={()=>setAttempt(value=>value+1)}>{gui.region_retry}</Button></p>}
  {items?.map(item=>item.who==='sys'?<p key={item.historyKey} className="text-caption">{item.text}</p>:item.who==='user'?
   <div key={item.historyKey} role="article" aria-label="你" className="py-row"><Markdown text={item.text}/></div>:
   <AgentContent key={item.historyKey} item={item} streaming={false} active={false} onOpenFile={onOpenFile}
    open={item.who==='tool'&&expanded.has(item.id)} onOpenChange={shown=>{if(item.who==='tool')setExpanded(now=>{const next=new Set(now);if(shown)next.add(item.id);else next.delete(item.id);return next;});}}/>)}
  <details className="text-caption text-muted-foreground"><summary>原始记录</summary>
   <HistoryRecords references={[reference]} exporter={exporter} showExportStatus={true}
    read={async(cursor,from,signal)=>{const source=historyApi(id,binding);try{return await source.record(cursor,from,signal);}finally{await source.dispose();}}}/>
  </details>
 </div>;
}
