import {IconClose,IconEye,IconFiles,IconPlus} from '@/components/icons';
import {FileKindIcon} from '@/components/FileKindIcon';
import {sizeWord} from '@/views/FilesPane';
import {cn} from '@/lib/utils';

export type DraftAttachment={key:string;name:string;size:number|null;thumb?:string|null;dir?:boolean;detail?:string;error?:string|null;busy?:boolean};
/** A file or directory is one full-width row. Preview is optional;
 * selection and sending never require entering a preview. */
export function AttachmentList({items,onOpen,onRemove,onAdd,onRetry,busy=false}:{
 items:DraftAttachment[];onOpen:(key:string)=>void;onRemove:(key:string)=>void;
 onAdd?:()=>void;onRetry?:(key:string)=>void;busy?:boolean;
}){
 return <div data-attachment-list className="flex min-w-0 flex-col">
  {items.map(item=><div key={item.key} data-files-selected={item.name} data-chat-pending-file={item.name}
   aria-busy={item.busy||undefined} title={item.error??item.detail}
   className="flex w-full min-w-0 items-center gap-in border-b text-callout aria-busy:animate-pulse motion-reduce:aria-busy:animate-none">
   <button type="button" data-files-card-preview data-chat-pending-preview aria-label={`预览 ${item.name}`} disabled={busy} onClick={()=>onOpen(item.key)}
    className="flex min-h-touch min-w-0 flex-1 items-center gap-row py-in text-left">
    {item.thumb?<img src={item.thumb} alt="" className="size-kind-mark flex-none rounded-md object-cover"/>:item.dir?<IconFiles className="size-kind-mark flex-none text-muted-foreground"/>:<FileKindIcon name={item.name} className="size-kind-mark flex-none"/>}
    <span className="min-w-0 flex-1"><span className="block truncate text-body">{item.name}</span>
     <span className={cn('block truncate text-caption tabular-nums',item.error?'text-destructive':'text-muted-foreground')}>{item.error?'发送失败':item.busy?'正在上传':[item.size===null?null:sizeWord(item.size),item.detail].filter(Boolean).join(' · ')}</span>
    </span>
    <span data-attachment-preview-glyph aria-hidden="true" title="预览" className="grid size-affordance flex-none place-items-center text-muted-foreground [&>svg]:size-affordance"><IconEye/></span>
   </button>
   {item.error&&onRetry&&<button type="button" disabled={busy} onClick={()=>onRetry(item.key)} className="min-h-touch px-in text-callout">重试</button>}
   <button type="button" data-chat-pending-drop aria-label={`移除 ${item.name}`} disabled={busy} onClick={()=>onRemove(item.key)}
    className="grid min-h-touch min-w-touch flex-none place-items-center text-muted-foreground"><IconClose className="size-4"/></button>
  </div>)}
  {onAdd&&<button type="button" data-attachment-add disabled={busy} onClick={onAdd}
   className="flex min-h-touch items-center gap-row text-left text-body text-muted-foreground"><IconPlus className="size-kind-mark"/>添加附件</button>}
 </div>;
}
