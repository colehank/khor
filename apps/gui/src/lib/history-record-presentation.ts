import type {HistoryRecordRef} from '@/gen/bindings/HistoryRecordRef';
import type {historySource} from './history-source';
import {completeHistoryRecord} from './history-export';
import {projectRecord,recordSegments} from './history-find-record';
import {recordString,limitedToolOutput} from './history-record-text';
import {paintHistory,type HistoryItem} from './history-paint';

type Source=Pick<ReturnType<typeof historySource>,'page'|'record'|'project'>;
const utf8=new TextEncoder();
/** Hydrate only fields selected by the canonical parser, never vendor JSON.
 * A mounted reader keeps one semantic record. Media has its own bounded
 * budget; a long text can explicitly request a larger bounded preview. */
export async function presentHistoryRecord(source:Source,reference:HistoryRecordRef,signal:AbortSignal,textLimit=1024*1024){
 const check=()=>{if(signal.aborted)throw new Error('已取消读取');};
 const ref=await completeHistoryRecord(source,reference,signal);check();
 const prepared=await projectRecord(ref,ref.cursor,{
  record:(cursor,from)=>{check();return source.record(cursor,from,signal);},
  project:(cursor,start,end,scaffold)=>{check();return source.project(cursor,start,end,scaffold,signal);},
 });check();
 if(!prepared||!prepared.projected.complete)throw new Error('这段内容暂无法展开，可导出原始记录');
 const entries=ref.item===null?prepared.projected.entries:prepared.projected.entries.filter(entry=>entry.key===`projection:${ref.item}`);
 if(!entries.length)throw new Error('这段记录没有可显示的内容');
 const expand=async function*(text:string){
  const parts=recordSegments(text,prepared.scaffold.strings);
  if(!parts)throw new Error('内容来源无法确认');
  for(const part of parts){
   if('text' in part){yield*part.text;continue;}
   for await(const char of recordString(part.range,prepared.read,prepared.account)){
    check();yield*part.escaped?JSON.stringify(char).slice(1,-1):char;
   }
  }
 };
 let retained=0;
 const restore=async(value:unknown,media=false,depth=0):Promise<unknown>=>{
  check();if(depth>128)throw new Error('内容层级过深');
  if(typeof value==='string'){
   const parts=recordSegments(value,prepared.scaffold.strings);
   if(!parts)throw new Error('内容来源无法确认');
   let result='',size=0;
   const add=(text:string)=>{
    const bytes=utf8.encode(text).length;size+=bytes;retained+=bytes;
    if(size>(media?24*1024*1024:textLimit)||retained>32*1024*1024)throw new Error('内容超过预览范围，可导出完整记录');
    result+=text;
   };
   for(const part of parts){
    if('text' in part){add(part.text);continue;}
    let chunk='';
    for await(const char of recordString(part.range,prepared.read,prepared.account)){
     check();chunk+=part.escaped?JSON.stringify(char).slice(1,-1):char;
     if(chunk.length>=4096){add(chunk);chunk='';}
    }
    if(chunk)add(chunk);
   }
   return result;
  }
  if(Array.isArray(value)){const out=[];for(const child of value)out.push(await restore(child,media,depth+1));return out;}
  if(value&&typeof value==='object'){
   const out:Record<string,unknown>={};
   for(const [key,child] of Object.entries(value))out[key]=await restore(child,media||key==='data'||key==='blob',depth+1);
   return out;
  }
  return value;
 };
 const items:HistoryItem[]=[];
 for(const item of paintHistory(entries)){
  const policy=prepared.projected.transforms.find(fact=>fact.kind==='tool_output'&&fact.item===Number(item.historyKey.split(':')[1]));
  const bounded=item.who==='tool'&&item.out!==null&&policy?.kind==='tool_output'
    ?{...item,out:await limitedToolOutput(expand(item.out),policy.max_bytes,policy.suffix)}:item;
  items.push(await restore(bounded) as HistoryItem);
 }
 check();return items;
}

/** Insert each source reference before its next painted contribution. Live
 * controls without a physical record retain their order after history. */
export function historyRecordSlots<T extends {historyRecords?:{record:HistoryRecordRef|null}[]}>(items:T[],references:HistoryRecordRef[]){
 const slots:HistoryRecordRef[][]=Array.from({length:items.length+1},()=>[]);
 let prior:HistoryRecordRef|null=null;
 for(const reference of references){
  const continuation=prior?.missing_end&&reference.missing_start&&prior.end===reference.start&&prior.item===reference.item;
  prior=reference;if(continuation)continue;
  const index=items.findIndex(item=>{
   const record=item.historyRecords?.find(part=>part.record)?.record;
   return record?record.start>=reference.end||(record.start===reference.start&&record.end===reference.end
    &&(reference.item===null||record.item===null||record.item>reference.item)):!item.historyRecords;
  });
  slots[index<0?items.length:index].push(reference);
 }
 return slots;
}
