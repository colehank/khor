import {useEffect,useRef,useState} from 'react';
import {inApp} from '@/api';
import {nativeFileWriter} from '@/lib/native-file-writer';
import {historyApi} from '@/lib/history-api';
import {browserDownload} from '@/lib/browser-download';
import {exportReadableHistory} from '@/lib/history-readable-export';
type Operation={controller:AbortController;source:ReturnType<typeof historyApi>};
/** Browser and native destinations consume the same canonical source. */
export function useReadableHistoryExport(id:string,binding:string|null,title:string){
 const owner=useRef<Operation|null>(null),[busy,setBusy]=useState(false),[notice,setNotice]=useState('');
 const retire=()=>{const active=owner.current;owner.current=null;if(active){active.controller.abort();void active.source.dispose();}};
 useEffect(()=>{setBusy(false);setNotice('');return()=>retire();},[id,binding]);
 const run=()=>{
  if(!binding||owner.current)return;
  const operation={controller:new AbortController(),source:historyApi(id,binding)};owner.current=operation;setBusy(true);setNotice('正在导出对话');
  void exportReadableHistory(operation.source,inApp?nativeFileWriter('conversation.md',operation.controller.signal):browserDownload('conversation.md','text/markdown;charset=utf-8'),title,operation.controller.signal).then(()=>{
   if(owner.current===operation)setNotice('对话已导出');
  },error=>{if(owner.current===operation)setNotice(operation.controller.signal.aborted?'已取消导出':String(error instanceof Error?error.message:error));}).finally(async()=>{
   await operation.source.dispose();if(owner.current===operation){owner.current=null;setBusy(false);}
  });
 };
 return {available:binding!==null,busy,notice,run,cancel:()=>{retire();setBusy(false);setNotice('已取消导出');}};
}
