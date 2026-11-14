/** FilePage and FilesPane own their insets and navigation. The task
 * overlay supplies only the shared surface, focus policy and gestures. */
import type {ReactNode} from 'react';
import {Dialog,DialogTitle,TaskOverlay} from '@/components/ui/dialog';
import {useWindowNarrow} from '@/hooks/use-narrow';

export function ContentOverlay({open,title,onClose,children,...attributes}:{open:boolean;title:string;onClose:()=>void;children:ReactNode}){
 const narrow=useWindowNarrow();
 return <Dialog open={open} onOpenChange={open=>{if(!open)onClose();}}>{open&&<TaskOverlay {...attributes} layout="content" purpose="preview" initialHeight="large" maxHeight="large" className="flex min-h-0 min-w-0 flex-col gap-0 overflow-hidden" style={{paddingInline:0,paddingBottom:0,paddingTop:narrow?'var(--touch)':0}} onOpenAutoFocus={event=>{event.preventDefault();(event.target as HTMLElement).focus({preventScroll:true});}} onCloseAutoFocus={event=>event.preventDefault()}>
  <DialogTitle className="sr-only">{title}</DialogTitle>
  {children}
 </TaskOverlay>}</Dialog>;
}
