import {useRef} from 'react';
import {IconSearch,IconClose} from '@/components/icons';
import {Input} from '@/components/ui/input';
import {gui} from '@/gen/catalog';
import {imeKey} from '@/lib/ime';
import {useFieldDismiss} from '@/hooks/use-field-dismiss';

/** Search has one clear button inside and an independent end-editing control outside. */
export function ContentSearch({label,value,onChange,onEditing,onEnter,onDismiss,autoFocus=true}: {label:string;value:string;onChange:(value:string)=>void;onEditing:(editing:boolean)=>void;onEnter?:()=>void;onDismiss?:()=>void;autoFocus?:boolean}) {
  const input=useRef<HTMLInputElement>(null),region=useRef<HTMLDivElement>(null);
  useFieldDismiss(onDismiss!==undefined,()=>[region.current],()=>onDismiss?.());
  return <div ref={region} className="content-search-field"><IconSearch/><Input ref={input} autoFocus={autoFocus} type="search" aria-label={label} placeholder={label} value={value} onFocus={()=>onEditing(true)} onBlur={()=>onEditing(false)} onChange={e=>onChange(e.target.value)} onKeyDown={e=>{if(imeKey(e))return;if(e.key==='Enter')onEnter?.();if(e.key==='Escape'){e.preventDefault();e.stopPropagation();input.current?.blur();onDismiss?.();}}}/>{value&&<button data-content-search-clear aria-label={gui.search_clear} onPointerDown={e=>e.preventDefault()} onClick={()=>{onChange('');input.current?.focus();}}><IconClose/></button>}</div>;
}
