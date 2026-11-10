import {useEffect,useRef,useState,type ReactNode} from 'react';
import {IconCopy,IconCheck} from '@/components/icons';
import {RowMenu} from '@/components/RowMenu';
import {Button} from '@/components/ui/button';
import {Dialog,TaskOverlay,TaskActions,DialogHeader,DialogTitle,DialogBody} from '@/components/ui/dialog';
import {gui} from '@/gen/catalog';
import {cn} from '@/lib/utils';

/** Message operations share one copy receipt and one menu on every chat face.
 * Editing is deliberately absent until the host can rewind its real context. */
export function MessageActions({text,children,align='start'}:{text:string;children:ReactNode;align?:'start'|'end'}){
 const [copy,setCopy]=useState<'idle'|'copied'|'failed'>('idle'),[selecting,setSelecting]=useState(false);
 const current=useRef(text);current.current=text;
 useEffect(()=>setCopy('idle'),[text]);
 const copyText=async()=>{
  try{if(!navigator.clipboard)throw new Error('Clipboard unavailable');await navigator.clipboard.writeText(text);if(current.current===text)setCopy('copied');}
  catch(error){if(current.current===text)setCopy('failed');throw error;}
 };
 const copyLabel=copy==='failed'?gui.copy_retry:copy==='copied'?gui.copied:gui.copy_text;
 return <>
  <RowMenu items={[
   {key:'copy',label:copyLabel,failed:copy==='failed',note:copy==='failed'?gui.copy_failed:undefined,run:copyText},
   {key:'select-text',label:gui.select_text,run:()=>setSelecting(true)},
  ]}>
   <div tabIndex={0} data-message-actions data-copy-state={copy} data-align={align} className="relative min-w-0 w-full rounded-md focus-visible:outline-2 focus-visible:outline-ring">
    {children}
    <div data-message-toolbar className={cn('mt-xxs flex h-ctl-md items-center',align==='end'&&'justify-end')}>
     <Button size="icon" variant="ghost" data-message-copy aria-label={copyLabel} title={copyLabel} onClick={()=>void copyText().catch(()=>{})}>
      {copy==='copied'?<IconCheck/>:<IconCopy/>}
     </Button>
    </div>
    <span className="sr-only" role="status">{copy==='copied'?gui.copied:copy==='failed'?gui.copy_failed:''}</span>
   </div>
  </RowMenu>
  <Dialog open={selecting} onOpenChange={setSelecting}>
   <TaskOverlay purpose="selection" scrollable>
    <DialogHeader><DialogTitle>{gui.select_text}</DialogTitle></DialogHeader>
    <DialogBody><div data-message-selection data-selectable tabIndex={0} className="whitespace-pre-wrap break-words text-read [touch-action:pan-y] [-webkit-touch-callout:default]">{text}</div></DialogBody>
    <TaskActions close={<Button onClick={()=>setSelecting(false)}>{gui.close}</Button>}/>
   </TaskOverlay>
  </Dialog>
 </>;
}
