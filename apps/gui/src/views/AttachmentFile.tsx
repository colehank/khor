import type {DeviceRow} from '@/api';
import {Dialog,TaskOverlay,DialogTitle} from '@/components/ui/dialog';
import type {Attachment} from '@/lib/term-attachments';
import {FilePage} from '@/views/FilePage';
export function AttachmentFile({entry,devices,onClose}:{entry:Attachment|null;devices:DeviceRow[];onClose:()=>void}){
 const device=entry?devices.find(d=>entry.from?d.name===entry.from:d.me):undefined;
 return <Dialog open={entry!==null} onOpenChange={open=>{if(!open)onClose();}}>{entry&&<TaskOverlay layout="content" purpose="preview" initialHeight="large" maxHeight="large" data-attachment-file className="flex min-h-0 flex-col gap-0 overflow-hidden p-0" onOpenAutoFocus={event=>{event.preventDefault();(event.target as HTMLElement).focus({preventScroll:true});}} onCloseAutoFocus={event=>event.preventDefault()}>
  <DialogTitle className="sr-only">{entry.name}</DialogTitle>
  <FilePage key={entry.key} machine={device?.name??entry.from??''} here={device?.me??entry.from===null} path={entry.path} localFile={entry.device?.files[0]?.file} more={false} onBack={onClose} onGone={onClose} onMoved={()=>{}}/>
 </TaskOverlay>}</Dialog>;
}
