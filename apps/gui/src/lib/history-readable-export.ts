import type {historySource} from './history-source';
import {checkedReplayPage} from './history-find-stream';
import {paintHistory,type HistoryItem} from './history-paint';
import {historyText} from './history-find-model';
import {recordText} from './history-find-record';
type Source=Pick<ReturnType<typeof historySource>,'page'|'record'|'project'>;
type Writer={write:(bytes:Uint8Array<ArrayBuffer>)=>Promise<void>;close:()=>Promise<void>;abort:()=>Promise<void>};
const encoder=new TextEncoder();
const labels:Record<string,string>={user:'你',agent:'智能体',thought:'思考',tool:'工具',plan:'计划',resource:'附件',sys:'状态',compaction:'上下文'};
const label=(who:string)=>labels[who]??'记录';
/** Export a frozen canonical source range, never the mounted/cache window.
 * Only cursor metadata is retained between passes; bytes commit atomically. */
export async function exportReadableHistory(source:Source,writer:Writer,title:string,signal:AbortSignal){
 let committed=false;
 const check=()=>{if(signal.aborted)throw new Error('已取消导出');};
 const write=async(text:string)=>{check();await writer.write(encoder.encode(text));};
 const read=async(cursor:string|null)=>{check();const page=checkedReplayPage(await source.page(cursor,false,signal));check();return page;};
 try{
  const pages:{cursor:string;first:string|null;last:string|null}[]=[],seen=new Set<string>();let cursor:string|null=null;
  for(;;){
   const page=await read(cursor);await new Promise(resolve=>setTimeout(resolve,0));check();pages.push({cursor:page.newer,first:page.entries[0]?.key??null,last:page.entries.at(-1)?.key??null});
   if(pages.length>4096)throw new Error('对话超过单次导出范围');
   if(page.older===null)break;
   if(seen.has(page.older)||page.older===cursor)throw new Error('历史读取未前进');seen.add(page.older);cursor=page.older;
  }
  await write(`# ${title.replace(/[\r\n]+/g,' ')}\n\n图片和附件保留名称与路径，文件本身不嵌入本文\n\n`);
  const records=new Set<string>();
  let previousText:{who:string;messageId?:string}|null=null;
  const itemText=async(item:HistoryItem,starts:boolean)=>{
   const body=historyText(item);if(!body.complete)throw new Error('部分内容无法完整读取，请单独导出原始记录');
   const messageId='messageId' in item?item.messageId:undefined;
   const continued='text' in item&&!starts&&previousText?.who===item.who
    &&(previousText.messageId===undefined||messageId===undefined||previousText.messageId===messageId);
   await write(`${continued?'':`\n\n## ${label(item.who)}\n\n`}${body.texts.join('\n\n')}`);
   previousText='text' in item?{who:item.who,messageId:messageId??(continued?previousText?.messageId:undefined)}:null;
  };
  for(const saved of pages.reverse()){
   const page=await read(saved.cursor);
   if((page.entries[0]?.key??null)!==saved.first||(page.entries.at(-1)?.key??null)!==saved.last)throw new Error('历史来源已变化，请重试');
   const items=paintHistory(page.entries,page.references);
   if(page.references.length&&items.some(item=>!item.historyRecords.some(source=>source.record)))throw new Error('无法确认原记录的顺序，请单独导出原始记录');
   const ordered=[...items.map(item=>({at:item.historyRecords.find(source=>source.record)?.record?.start??Infinity,item,reference:null})),
    ...page.references.map(reference=>({at:reference.start,item:null,reference}))].sort((a,b)=>a.at-b.at);
   let lastEntry=-1;
   const barrier=(from:number,to:number)=>page.entries.slice(from,to).some(entry=>entry.frame.kind==='turn'||entry.frame.kind==='capture_gap');
   for(const entry of ordered){
    if(entry.item){
     const indexes=entry.item.historyEntries.map(key=>page.entries.findIndex(source=>source.key===key)).filter(index=>index>=0);
     const first=Math.min(...indexes);
     if(barrier(lastEntry+1,first))previousText=null;
     await itemText(entry.item,!!page.entries[first]?.starts_message);
     lastEntry=Math.max(lastEntry,...indexes);continue;
    }
    previousText=null;
    const reference=entry.reference!;const key=`${reference.start}:${reference.end}:${reference.item??'record'}`;
    if(records.has(key))continue;records.add(key);
    if(reference.missing_start||reference.missing_end)throw new Error('原记录尚不完整，请稍后导出');
    const stream=recordText(reference,page.newer,{record:(cursor,from)=>source.record(cursor,from,signal),project:(cursor,start,end,scaffold)=>source.project(cursor,start,end,scaffold,signal)});let result=await stream.next();
    while(!result.done){
     const part=result.value;await write(`\n\n## ${label(part.who)}\n\n`);let buffer='';
     for await(const text of part.text){check();buffer+=text;if(buffer.length>=16384){await write(buffer);buffer='';}}
     await write(buffer+'\n\n');if(!part.complete)throw new Error('部分内容无法完整读取，请单独导出原始记录');
     result=await stream.next();
    }
    if(!result.value)throw new Error('部分内容无法完整读取，请单独导出原始记录');
   }
   if(barrier(lastEntry+1,page.entries.length))previousText=null;
   await new Promise(resolve=>setTimeout(resolve,0));check();
  }
  await write("\n");await writer.close();committed=true;
 }finally{if(!committed)await writer.abort().catch(()=>{});}
}
