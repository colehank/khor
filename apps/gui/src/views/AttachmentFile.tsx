import type {DeviceRow} from '@/api';
import {ContentOverlay} from '@/components/ContentOverlay';
import type {Attachment} from '@/lib/term-attachments';
import {FilePage} from '@/views/FilePage';
export function AttachmentFile({entry,devices,onClose}:{entry:Attachment|null;devices:DeviceRow[];onClose:()=>void}){
 const device=entry?devices.find(d=>entry.from?d.name===entry.from:d.me):undefined;
 return <ContentOverlay open={entry!==null} title={entry?.name??''} onClose={onClose} data-attachment-file>{entry&&
  <FilePage key={entry.key} machine={device?.name??entry.from??''} here={device?.me??entry.from===null} path={entry.path} localFile={entry.device?.files[0]?.file} more={false} onBack={onClose} onGone={onClose} onMoved={()=>{}}/>
 }</ContentOverlay>;
}
