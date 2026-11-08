import {useState} from 'react';
import {Button} from '@/components/ui/button';
/** Copy failures stay actionable; a success label never precedes the receipt. */
export function CopyText({text}:{text:string|(()=>string)}){
 const [state,setState]=useState<'idle'|'copied'|'failed'>('idle');
 return <><Button size="sm" variant="ghost" data-copy-text onClick={()=>{
  void navigator.clipboard.writeText(typeof text==='function'?text():text).then(()=>setState('copied'),()=>setState('failed'));
 }}>{state==='copied'?'已复制':state==='failed'?'重试复制':'复制'}</Button><span className="sr-only" role="status">{state==='copied'?'已复制':state==='failed'?'复制失败':''}</span></>;
}
