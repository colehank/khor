import {useState} from 'react';
import {Button} from '@/components/ui/button';
import {gui} from '@/gen/catalog';
/** Copy failures stay actionable; a success label never precedes the receipt. */
export function CopyText({text}:{text:string|(()=>string)}){
 const [state,setState]=useState<'idle'|'copied'|'failed'>('idle');
 return <><Button size="sm" variant="ghost" data-copy-text onClick={()=>{
  void (navigator.clipboard?.writeText(typeof text==='function'?text():text)??Promise.reject(new Error('Clipboard unavailable'))).then(()=>setState('copied'),()=>setState('failed'));
 }}>{state==='copied'?gui.copied:state==='failed'?gui.copy_retry:gui.copy_text}</Button><span className="sr-only" role="status">{state==='copied'?gui.copied:state==='failed'?gui.copy_failed:''}</span></>;
}
