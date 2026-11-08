import {useEffect,useMemo,useState} from 'react';
import {inApp} from '@/api';
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
 const home=useLinkHome(),path=resourceFilePath(item.uri),name=item.name||'附件';
 useEffect(()=>{setFailed(false);setOpen(false);},[source,item.uri]);
 if(source&&item.kind==='audio'&&!failed)return <audio data-chat-audio controls preload="none" aria-label={name} src={source} onError={()=>setFailed(true)}/>;
 if(source&&item.kind==='image'&&!failed)return <>
  <button type="button" data-chat-media-preview aria-label={`查看图片：${name}`} className="block max-w-full rounded-md" onClick={()=>setOpen(true)}>
   <img data-chat-image loading="lazy" className="max-h-[60vh] max-w-full rounded-md object-contain" alt={name} src={source} onError={()=>setFailed(true)}/>
  </button>
  <Dialog open={open} onOpenChange={setOpen}><DialogContent purpose="preview" scrollable><DialogHeader><DialogTitle>{name}</DialogTitle></DialogHeader>
   <DialogBody className="overflow-hidden"><img className="h-full min-h-0 w-full object-contain" alt={name} src={source}/></DialogBody>
   <DialogFooter>{path&&<Button size="sm" onClick={()=>onOpenFile(path)}>在文件视图中打开</Button>}
    {!inApp&&<a className="text-callout" href={source} download={name}>存储图片</a>}
    <Button size="sm" variant="ghost" onClick={()=>setOpen(false)}>{gui.close}</Button>
   </DialogFooter>
  </DialogContent></Dialog>
 </>;
 return <div data-chat-resource className="rounded-md bg-fill px-row py-in text-callout">
  <span>{name}</span>{failed&&<p role="status" className="text-caption text-muted-foreground">附件无法打开</p>}
  {path?<Button size="sm" variant="ghost" onClick={()=>onOpenFile(path)}>打开文件</Button>:item.uri&&/^https?:\/\//i.test(item.uri)?<Button size="sm" variant="ghost" onClick={event=>{
    const url=item.uri!;assertPageUrl(url);if(linkIntent(event)==='system')void openSystemUrl(url).catch(()=>setFailed(true));else if(home?.openPage)home.openPage(url);else void openSystemUrl(url).catch(()=>setFailed(true));
  }}>打开链接</Button>:<p className="text-caption text-muted-foreground">{item.data?'此媒体暂无法预览':'源内容不可用'}</p>}
 </div>;
}
