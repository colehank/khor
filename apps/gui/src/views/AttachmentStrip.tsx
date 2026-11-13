import {IconClose, IconFiles, IconPlus} from '@/components/icons';
import {FileKindIcon} from '@/components/FileKindIcon';
import {sizeWord} from '@/views/FilesPane';
import {cn} from '@/lib/utils';

export type DraftAttachment = {key:string;name:string;size:number|null;thumb?:string|null;dir?:boolean;error?:string|null;busy?:boolean};
/** One horizontal draft, next to its conversation. Source browsing lives
 * in the chooser; these cards only preview, remove, or retry an item. */
export function AttachmentStrip({items,onOpen,onRemove,onAdd,onRetry,busy=false}:{
 items:DraftAttachment[];onOpen:(key:string)=>void;onRemove:(key:string)=>void;
 onAdd?:()=>void;onRetry?:(key:string)=>void;busy?:boolean;
}){
 return <div data-attachment-strip className="flex min-w-0 items-stretch gap-in overflow-x-auto overscroll-x-contain py-in">
  {items.map(item=><div key={item.key} data-files-selected={item.name} data-chat-pending-file={item.name}
   aria-busy={item.busy||undefined} title={item.error??undefined}
   className={cn("relative flex w-48 max-w-full flex-none items-center rounded-lg bg-fill pe-touch text-callout aria-busy:animate-pulse motion-reduce:aria-busy:animate-none",item.error&&onRetry&&"pb-touch")}>
   <button type="button" data-files-card-preview data-chat-pending-preview disabled={busy} onClick={()=>onOpen(item.key)}
    className="flex min-h-touch min-w-0 flex-1 items-center gap-in p-in text-left">
    {item.thumb?<img src={item.thumb} alt="" className="size-10 flex-none rounded-md object-cover"/>:item.dir?<IconFiles className="size-7 flex-none text-muted-foreground"/>:<FileKindIcon name={item.name} className="size-7 flex-none"/>}
    <span className="min-w-0 flex-1"><span className="block truncate text-body">{item.name}</span>
     <span className={cn('block truncate text-caption tabular-nums',item.error?'text-destructive':'text-muted-foreground')}>{item.error?'发送失败':item.busy?'正在上传':item.size===null?'':sizeWord(item.size)}</span>
    </span>
   </button>
   <button type="button" data-chat-pending-drop aria-label={`移除 ${item.name}`} disabled={busy} onClick={()=>onRemove(item.key)}
    className="absolute right-0 top-0 grid min-h-touch min-w-touch place-items-center text-muted-foreground"><IconClose className="size-4"/></button>
   {item.error&&onRetry&&<button type="button" disabled={busy} onClick={()=>onRetry(item.key)} className="absolute bottom-0 right-0 min-h-touch px-in text-callout">重试</button>}
  </div>)}
  {onAdd&&<button type="button" data-attachment-add aria-label="继续添加附件" disabled={busy} onClick={onAdd}
   className="grid min-h-touch min-w-touch flex-none place-items-center self-stretch rounded-lg border border-dashed text-muted-foreground"><IconPlus className="size-5"/></button>}
 </div>;
}
