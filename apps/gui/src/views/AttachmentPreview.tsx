import {useEffect,useRef,useState} from 'react';
import {Dialog,TaskOverlay,DialogTitle,TaskActions} from '@/components/ui/dialog';
import {Button} from '@/components/ui/button';
import {FileKindIcon} from '@/components/FileKindIcon';
import {sizeWord} from '@/views/FilesPane';

/** Browser-owned drafts are previewed locally. Saving creates a new File;
 * the chosen source is never overwritten and no target upload is started. */
export function AttachmentPreview({file,onClose,onSave}:{file:File|null;onClose:()=>void;onSave?:(file:File)=>void}){
 const title=useRef<HTMLHeadingElement>(null);
 const retained=useRef<File|null>(file);if(file)retained.current=file;
 const current=file??retained.current;const [,repaint]=useState(0);
 const [url,setUrl]=useState(''),[name,setName]=useState(''),[text,setText]=useState<string|null>(null),[original,setOriginal]=useState<string|null>(null),[error,setError]=useState('');
 useEffect(()=>{
  if(!current){setUrl('');return;}
  const file=current;setName(file.name);setText(null);setOriginal(null);setError('');
  const url=URL.createObjectURL(file);setUrl(url);let live=true;
  const readable=file.size<=1024*1024&&(file.type.startsWith('text/')||/\.(txt|md|json|csv|yaml|yml|toml|log|ts|tsx|js|py|rs|css)$/i.test(file.name));
  if(readable)void file.text().then(value=>{if(live){setText(value);setOriginal(value);}},()=>{if(live)setError('无法读取此文件');});
  return()=>{live=false;URL.revokeObjectURL(url);};
 },[current]);
 const dirty=file!==null&&(name!==file.name||text!==original);
 const valid=!!name.trim()&&!/[\\/\0]/.test(name)&&name!=='.'&&name!=='..';
 const save=()=>{if(file&&valid&&onSave){onSave(new File([text===null||text===original?file:text],name.trim(),{type:file.type,lastModified:Date.now()}));onClose();}};
 return <Dialog open={file!==null} onOpenChange={open=>{if(!open)onClose();}}><TaskOverlay purpose={onSave?'edit':'preview'} initialHeight="large" maxHeight="large" scrollable data-attachment-preview onOpenAutoFocus={event=>{event.preventDefault();title.current?.focus({preventScroll:true});}} onCloseAutoFocus={()=>{retained.current=null;repaint(value=>value+1);}}>
  <DialogTitle ref={title} tabIndex={-1}>附件预览</DialogTitle>
  <TaskActions dirty={dirty} cancel={onSave?<Button onClick={onClose}>取消</Button>:undefined} close={!onSave?<Button onClick={onClose}>关闭</Button>:undefined}
   primary={onSave?<Button data-attachment-save variant="primary" disabled={!valid} onClick={save}>{dirty?'保存修改':'完成'}</Button>:undefined}/>
  <label className="flex min-w-0 items-center gap-row text-body"><span className="flex-none">文件名</span><input data-attachment-name aria-label="文件名" value={name} readOnly={!onSave} onChange={event=>setName(event.target.value)} className="min-w-0 flex-1 rounded-md bg-fill px-in py-in"/></label>
  {current&&<p className="text-callout tabular-nums text-muted-foreground">{sizeWord(current.size)}</p>}
  {error&&<p role="alert" className="text-body text-destructive">{error}</p>}
  <div data-attachment-preview-content className="flex min-h-0 flex-1 flex-col overflow-auto rounded-lg bg-fill">
   {current?.type.startsWith('image/')?<img src={url} alt={name} className="min-h-0 max-w-full flex-1 object-contain"/>:
    current?.type.startsWith('video/')?<video src={url} controls playsInline className="min-h-0 w-full flex-1"/>:
    current?.type.startsWith('audio/')?<audio src={url} controls className="w-full"/>:
    text!==null?<textarea data-attachment-text aria-label="文件内容" value={text} readOnly={!onSave} onChange={event=>setText(event.target.value)} className="min-h-0 w-full flex-1 resize-none bg-transparent p-row font-mono text-body"/>:
    current?.type==='application/pdf'?<iframe title={name} src={url} className="min-h-0 w-full flex-1 border-0"/>:
    <div className="flex flex-1 flex-col items-center justify-center gap-row p-row text-body text-muted-foreground"><FileKindIcon name={name} className="size-10"/><span>此类型可作为附件发送</span></div>}
  </div>
 </TaskOverlay></Dialog>;
}
