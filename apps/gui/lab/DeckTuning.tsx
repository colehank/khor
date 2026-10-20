import {useEffect,useId,useState} from 'react';
import {Button} from '@/components/ui/button';
import {Dialog,TaskOverlay,DialogBody,DialogTitle,DialogFooter} from '@/components/ui/dialog';
import {DEFAULT_TUNING,tuningFields,type DeckTuning,type TuningKey} from './deck-tuning';
type Props={value:DeckTuning;onChange:(patch:Partial<DeckTuning>)=>void;disabled?:boolean};
function Field({field,value,onChange,disabled}:Props&{field:typeof tuningFields[number]}){
 const id=useId(),n=value[field.key],min=field.key==='dwellSlowMs'?Math.max(field.min,value.dwellFastMs):field.min;
 const max=field.key==='dwellFastMs'?Math.min(field.max,value.dwellSlowMs):field.max;
 const [draft,setDraft]=useState(String(n));useEffect(()=>setDraft(String(n)),[n]);
 const commit=()=>{const num=Number(draft);if(draft.trim()&&Number.isFinite(num)){const next=Math.max(min,Math.min(max,num));setDraft(String(next));onChange({[field.key]:next});}else setDraft(String(n));};
 return <div className="deck-tuning-field" data-sheet-hold>
  <div className="deck-tuning-label"><label htmlFor={id}>{field.label}</label><input id={id} type="number" disabled={disabled} inputMode={field.step<1?'decimal':'numeric'} min={min} max={max} step={field.step} value={draft} onChange={e=>{const text=e.target.value;setDraft(text);const num=Number(text);if(text.trim()&&Number.isFinite(num)&&num>=min&&num<=max)onChange({[field.key]:num});}} onBlur={commit} onKeyDown={e=>{if(e.key==='Enter')e.currentTarget.blur();}}/><span>{field.unit}</span></div>
  <input type="range" disabled={disabled} aria-label={field.label+'滑条'} min={min} max={max} step={field.step} value={n} onChange={e=>onChange({[field.key]:Number(e.target.value)})}/>
  <p>{field.hint}</p>
 </div>;
}
export function TuningFields({value,onChange,advanced=false,disabled}:Props&{advanced?:boolean}){
 return <div className="deck-tuning-fields">{tuningFields.filter((_,i)=>advanced?i>=2:i<2).map(field=><Field key={field.key} field={field} value={value} onChange={onChange} disabled={disabled}/>)}</div>;
}
export function DeckTuningDialog({value,onChange,open,onOpenChange}:Props&{open:boolean;onOpenChange:(open:boolean)=>void}){
 const [copied,setCopied]=useState('复制参数');
 const change=(patch:Partial<Record<TuningKey,number>>)=>{setCopied('复制参数');onChange(patch);};
 return <Dialog open={open} onOpenChange={onOpenChange}><TaskOverlay purpose="edit" initialHeight="large" maxHeight="large" scrollable onOpenAutoFocus={e=>e.preventDefault()}>
  <DialogTitle>药丸手感</DialogTitle>
  <DialogBody><p className="text-callout text-muted-foreground">下一次触碰生效 · 仅保存到这个浏览器的 Lab</p><TuningFields value={value} onChange={change}/>
   <details className="deck-tuning-more"><summary>更多参数</summary><TuningFields value={value} onChange={change} advanced/></details>
  </DialogBody>
  <DialogFooter><Button onClick={()=>change(DEFAULT_TUNING)}>恢复默认</Button><Button onClick={async()=>{try{await navigator.clipboard.writeText(JSON.stringify(value,null,2));setCopied('已复制');}catch{setCopied('复制失败');}}}>{copied}</Button><Button onClick={()=>onOpenChange(false)}>完成</Button></DialogFooter>
 </TaskOverlay></Dialog>;
}
