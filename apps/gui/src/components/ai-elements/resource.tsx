import {useEffect,useMemo,useRef,useState} from 'react';
import {downloadableResource,saveResource} from '@/lib/resource-download';
import type {AcpResource} from '@/lib/acp-view';
import {inlineMediaSource,resourceFilePath} from '@/lib/chat-media';
import {Button} from '@/components/ui/button';
import {Dialog,DialogContent,DialogHeader,DialogTitle,DialogBody,DialogFooter} from '@/components/ui/dialog';
import {gui} from '@/gen/catalog';
import {useLinkHome} from '@/views/linkHome';
import {linkIntent} from '@/lib/content-link-intent';
import {assertPageUrl,openSystemUrl} from '@/lib/client-opener';
export function AgentResource({item,onOpenFile}:{item:AcpResource;onOpenFile:(path:string)=>void}){
 const source=useMemo(()=>inlineMediaSource(item),[item.kind,item.mime,item.data]);
 const [open,setOpen]=useState(false),[failed,setFailed]=useState(false);
 const saving=useRef<AbortController|null>(null),[busy,setBusy]=useState(false),[saveError,setSaveError]=useState('');
 const save=()=>{if(saving.current||!item.data)return;const owner=new AbortController();saving.current=owner;setBusy(true);setSaveError('');
  void saveResource(item.data,item.mime,owner.signal).catch(error=>{if(!owner.signal.aborted)setSaveError(error instanceof Error?error.message:String(error));}).finally(()=>{if(saving.current===owner){saving.current=null;setBusy(false);}});};
 const saveButton=downloadableResource(item.data)&&<Button size="sm" variant="ghost" disabled={busy} onClick={save}>{busy?'正在准备':'存储附件'}</Button>;
 const home=useLinkHome(),path=resourceFilePath(item.uri),name=item.name||'附件';
 useEffect(()=>{setFailed(false);setOpen(false);setSaveError('');setBusy(false);return()=>{saving.current?.abort();saving.current=null;};},[source,item.uri,item.data]);
 if(source&&item.kind==='audio'&&!failed)return <div><audio data-chat-audio controls preload="none" aria-label={name} src={source} onError={()=>setFailed(true)}/>{saveButton}{saveError&&<p role="alert" className="text-caption text-destructive">{saveError}</p>}</div>;
 if(source&&item.kind==='image'&&!failed)return <>
  <button type="button" data-chat-media-preview aria-label={`查看图片：${name}`} className="block max-w-full rounded-md" onClick={()=>setOpen(true)}>
   <img data-chat-image loading="lazy" className="max-h-[60vh] max-w-full rounded-md object-contain" alt={name} src={source} onError={()=>setFailed(true)}/>
  </button>
  <Dialog open={open} onOpenChange={setOpen}><DialogContent purpose="preview" scrollable><DialogHeader><DialogTitle>{name}</DialogTitle></DialogHeader>
   <DialogBody className="overflow-hidden"><img className="h-full min-h-0 w-full object-contain" alt={name} src={source}/></DialogBody>
   <DialogFooter>{path&&<Button size="sm" onClick={()=>onOpenFile(path)}>在文件视图中打开</Button>}
    {saveButton}{saveError&&<p role="alert" className="text-caption text-destructive">{saveError}</p>}
    <Button size="sm" variant="ghost" onClick={()=>setOpen(false)}>{gui.close}</Button>
   </DialogFooter>
  </DialogContent></Dialog>
 </>;
 return <div data-chat-resource className="rounded-md bg-fill px-row py-in text-callout">
  <span>{name}</span>{saveButton}{saveError&&<p role="alert" className="text-caption text-destructive">{saveError}</p>}{failed&&<p role="status" className="text-caption text-muted-foreground">附件无法打开</p>}
  {path?<Button size="sm" variant="ghost" onClick={()=>onOpenFile(path)}>打开文件</Button>:item.uri&&/^https?:\/\//i.test(item.uri)?<Button size="sm" variant="ghost" onClick={event=>{
    const url=item.uri!;assertPageUrl(url);if(linkIntent(event)==='system')void openSystemUrl(url).catch(()=>setFailed(true));else if(home?.openPage)home.openPage(url);else void openSystemUrl(url).catch(()=>setFailed(true));
  }}>打开链接</Button>:<p className="text-caption text-muted-foreground">{item.data?'此媒体暂无法预览':'源内容不可用'}</p>}
 </div>;
}
