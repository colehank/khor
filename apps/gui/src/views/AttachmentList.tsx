import {IconPlus} from '@/components/icons';
import {FileChoiceRow} from '@/views/FilesPane';

export type DraftAttachment={key:string;name:string;size:number|null;thumb?:string|null;dir?:boolean;detail?:string;error?:string|null;busy?:boolean};
/** A file or directory is one full-width row. Preview is optional;
 * selection and sending never require entering a preview. */
export function AttachmentList({items,onOpen,onRemove,onAdd,onRetry,busy=false}:{
 items:DraftAttachment[];onOpen:(key:string)=>void;onRemove:(key:string)=>void;
 onAdd?:()=>void;onRetry?:(key:string)=>void;busy?:boolean;
}){
 return <div data-attachment-list className="flex min-w-0 flex-col">
  {items.map(item=><div key={item.key} data-files-selected={item.name} data-chat-pending-file={item.name}>
   <FileChoiceRow name={item.name} dir={!!item.dir} size={item.size} detail={item.error?'发送失败':item.detail} selected disabled={busy||item.busy} onEnter={()=>item.dir?onOpen(item.key):onRemove(item.key)} onChoose={()=>onRemove(item.key)} onOpen={()=>onOpen(item.key)}/>
   {item.error&&onRetry&&<button type="button" disabled={busy} onClick={()=>onRetry(item.key)} className="min-h-touch px-in text-callout">重试</button>}
  </div>)}
  {onAdd&&<button type="button" data-attachment-add disabled={busy} onClick={onAdd} className="flex min-h-touch items-center gap-in text-left text-body text-muted-foreground"><IconPlus className="size-kind-mark"/>添加附件</button>}
 </div>;
}
