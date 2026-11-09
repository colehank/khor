import {useEffect,useLayoutEffect,useRef,useState} from "react";
import {IconChevronDown,IconChevronUp,IconClose} from "@/components/icons";
import {Button} from "@/components/ui/button";
import {Input} from "@/components/ui/input";
import {PopupRow} from "@/components/settings/PopupRow";
import {SwitchRow} from "@/components/settings/SwitchRow";
import {gui} from "@/gen/catalog";
import {baseKeyCaption,strokeCaption} from "@/lib/term-key-codec";
import {BASE_KEYS,KEY_ROW_LIMITS,type CustomKeyEntry} from "@/lib/term-key-model";
import {completeKeyboardStroke,emptyKeyboardStroke,keyboardGroup,type KeyboardStroke} from "@/lib/term-keyboard-layout";
import {TerminalKeyboard} from "@/views/TerminalKeyboard";

export function KeyRowEntryEditor({entry,onSave,onDelete,formId,onReady}: {
  entry:CustomKeyEntry|null;onSave:(entry:CustomKeyEntry)=>void;onDelete?:()=>void;
  formId:string;onReady:(ready:boolean)=>void;
}) {
  const [id]=useState(()=>entry?.id??`custom:${crypto.randomUUID()}`);
  const [label,setLabel]=useState(entry?.label??"");
  const [steps,setSteps]=useState<KeyboardStroke[]>(()=>entry?entry.kind==="chord"?[{...entry.stroke}]:entry.steps.map(step=>({...step})):[emptyKeyboardStroke()]);
  const [active,setActive]=useState(0),[keyboard,setKeyboard]=useState(false),[error,setError]=useState<string|null>(null);
  const stepStrip=useRef<HTMLDivElement>(null);
  useEffect(()=>{
    const strip=stepStrip.current,button=strip?.querySelector<HTMLElement>(`[data-key-step-select="${active}"]`);
    if(!strip||!button)return;
    const a=strip.getBoundingClientRect(),b=button.getBoundingClientRect();
    if(b.left<a.left)strip.scrollLeft-=a.left-b.left;else if(b.right>a.right)strip.scrollLeft+=b.right-a.right;
  },[active,steps.length]);
  const complete=steps.map(completeKeyboardStroke),ready=complete.every(Boolean)&&[...label.trim()].length<=KEY_ROW_LIMITS.labelCharacters&&new TextEncoder().encode(label.trim()).length<=KEY_ROW_LIMITS.labelBytes;
  useLayoutEffect(()=>onReady(ready),[ready,onReady]);
  const current=steps[active]??steps[0],picked=completeKeyboardStroke(current);
  const update=(next:KeyStrokeUpdate)=>{setSteps(previous=>previous.map((step,index)=>index===active?{...step,...next}:step));setError(null);};
  const move=(index:number,by:number)=>{const next=[...steps];[next[index],next[index+by]]=[next[index+by],next[index]];setSteps(next);setActive(index+by);};
  const save=()=>{if(!ready)return;try{const action=complete.length===1?{kind:"chord" as const,stroke:complete[0]!}:{kind:"sequence" as const,steps:complete.map(step=>step!)};onSave({id,on:entry?.on??true,label:label.trim(),...action});}catch{setError(gui.key_row_invalid);}};
  const group=(key:string)=>({letters:gui.key_row_letters,symbols:gui.keys_group_symbol,function:gui.key_row_function,navigation:gui.nav})[keyboardGroup(key)];
  return <form id={formId} data-key-entry-editor onSubmit={event=>{event.preventDefault();save();}} className="flex min-w-0 flex-col gap-row">
    <div data-key-entry-result data-key-entry-preview className="sticky top-0 z-10 flex min-w-0 flex-col gap-in bg-background py-in">
      <div ref={stepStrip} className="flex min-w-0 items-center gap-in overflow-x-auto" data-key-entry-steps>
        {steps.map((_step,index)=><div key={index} data-key-entry-step={index} className="flex min-w-0 flex-none items-center gap-xxs">
          {index>0&&<span aria-hidden className="text-muted-foreground">→</span>}
          <button type="button" data-key-step-select={index} aria-pressed={active===index} className={`min-h-touch rounded-sm px-in text-body ${active===index?"bg-fill-strong":"bg-fill"}`} onClick={()=>setActive(index)}>{complete[index]?strokeCaption(complete[index]!):gui.key_row_choose}</button>
          {steps.length>1&&<><button type="button" data-key-step-remove aria-label={gui.key_row_remove_step} className="grid min-h-touch min-w-touch place-items-center" onClick={()=>{setSteps(steps.filter((_,at)=>at!==index));setActive(active>index?active-1:active===index?Math.max(0,index-1):active);}}><IconClose className="size-affordance"/></button>
          {index>0&&<button type="button" data-key-step-up aria-label={gui.key_row_move_up} className="grid min-h-touch min-w-touch place-items-center" onClick={()=>move(index,-1)}><IconChevronUp className="size-affordance"/></button>}
          {index<steps.length-1&&<button type="button" data-key-step-down aria-label={gui.key_row_move_down} className="grid min-h-touch min-w-touch place-items-center" onClick={()=>move(index,1)}><IconChevronDown className="size-affordance"/></button>}</>}
        </div>)}
        {steps.length<KEY_ROW_LIMITS.steps&&<Button type="button" className="flex-none" data-key-step-add disabled={!picked} onClick={()=>{setSteps([...steps,emptyKeyboardStroke()]);setActive(steps.length);}}>{gui.key_row_add_step}</Button>}
      </div>
      {label.trim()&&<span className="break-words text-body font-title">{label.trim()}</span>}
    </div>
    <div data-settings-card className="flex min-w-0 flex-col">
      <PopupRow name="key-entry-base" label={gui.key_row_base_key} value={current.key??""} choices={[
        {value:"",label:gui.key_row_choose},...BASE_KEYS.map(key=>({value:key,label:baseKeyCaption(key),group:group(key)})).sort((a,b)=>a.group.localeCompare(b.group)),
      ]} onPick={key=>update({key:key?key as KeyboardStroke["key"]:null})}/>
      {([['ctrl','Ctrl'],['alt','Alt / ⌥'],['shift','Shift / ⇧']] as const).map(([modifier,name])=><SwitchRow key={modifier} name={`key-entry-${modifier}`} label={name} on={current[modifier]} onToggle={()=>update({[modifier]:!current[modifier]})}/>)}
      <div data-setting-row="value" className="flex min-w-0 items-center gap-in px-row py-in"><label htmlFor={`${formId}-name`} className="flex-none text-body">{gui.key_row_name}</label><Input id={`${formId}-name`} data-key-entry-name value={label} maxLength={KEY_ROW_LIMITS.labelCharacters*2} placeholder={gui.key_row_name_auto} onChange={event=>{setLabel(event.target.value);setError(null);}} className="min-w-0 flex-1"/></div>
    </div>
    <div className="flex flex-wrap gap-in"><Button type="button" data-key-show-keyboard aria-expanded={keyboard} onClick={()=>setKeyboard(!keyboard)}>{keyboard?gui.keyboard_hide:gui.keyboard_show}</Button>{current.key&&<Button type="button" data-key-entry-clear variant="ghost" onClick={()=>update({key:null})}>{gui.key_row_clear}</Button>}</div>
    {keyboard&&<TerminalKeyboard stroke={current} onChange={stroke=>update(stroke)}/>}
    {error&&<p role="alert" className="text-callout text-destructive">{error}</p>}
    {onDelete&&<Button type="button" data-key-entry-delete className="min-h-touch self-start" variant="ghost" onClick={onDelete}>{gui.delete}</Button>}
  </form>;
}
type KeyStrokeUpdate=Partial<KeyboardStroke>;
