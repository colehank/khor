import {useEffect,useLayoutEffect,useRef,useState} from "react";
import {IconMore,IconPlus} from "@/components/icons";
import {Button} from "@/components/ui/button";
import {Input} from "@/components/ui/input";
import {DropdownMenu,DropdownMenuTrigger,DropdownMenuContent,DropdownMenuItem,DropdownMenuLabel,DropdownMenuSeparator} from "@/components/ui/dropdown-menu";
import {PopupRow} from "@/components/settings/PopupRow";
import {SwitchRow} from "@/components/settings/SwitchRow";
import {gui} from "@/gen/catalog";
import {baseKeyCaption,strokeCaption} from "@/lib/term-key-codec";
import {BASE_KEYS,KEY_ROW_LIMITS,type CustomKeyEntry} from "@/lib/term-key-model";
import {completeKeyboardStroke,emptyKeyboardStroke,keyboardGroup,type KeyboardStroke} from "@/lib/term-keyboard-layout";
import {TerminalKeyboard} from "@/views/TerminalKeyboard";

export function KeyRowEntryEditor({entry,onSave,onDelete,formId,onReady,onDirty}: {
  entry:CustomKeyEntry|null;onSave:(entry:CustomKeyEntry)=>void;onDelete?:()=>void;
  formId:string;onReady:(ready:boolean)=>void;onDirty?:(dirty:boolean)=>void;
}) {
  const [id]=useState(()=>entry?.id??`custom:${crypto.randomUUID()}`);
  const [label,setLabel]=useState(entry?.label??"");
  const [steps,setSteps]=useState<KeyboardStroke[]>(()=>entry?entry.kind==="chord"?[{...entry.stroke}]:entry.steps.map(step=>({...step})):[emptyKeyboardStroke()]);
  const [start]=useState(()=>JSON.stringify({label:entry?.label??"",steps}));
  const dirty=JSON.stringify({label,steps})!==start;
  useLayoutEffect(()=>onDirty?.(dirty),[dirty,onDirty]);
  const [active,setActive]=useState(0),[keyboard,setKeyboard]=useState(true),[error,setError]=useState<string|null>(null);
  const stepStrip=useRef<HTMLDivElement>(null);
  useEffect(()=>{
    const strip=stepStrip.current,button=strip?.querySelector<HTMLElement>(`[data-key-step-select="${active}"]`);
    if(!strip||!button)return;
    const a=strip.getBoundingClientRect(),b=button.getBoundingClientRect();
    if(b.left<a.left)strip.scrollLeft-=a.left-b.left;else if(b.right>a.right)strip.scrollLeft+=b.right-a.right;
  },[active,steps.length]);
  const complete=steps.map(completeKeyboardStroke),allComplete=complete.every(Boolean),ready=allComplete&&[...label.trim()].length<=KEY_ROW_LIMITS.labelCharacters&&new TextEncoder().encode(label.trim()).length<=KEY_ROW_LIMITS.labelBytes;
  useLayoutEffect(()=>onReady(ready),[ready,onReady]);
  const current=steps[active]??steps[0];
  const update=(next:KeyStrokeUpdate)=>{setSteps(previous=>previous.map((step,index)=>index===active?{...step,...next}:step));setError(null);};
  const move=(index:number,by:number)=>{if(index+by<0||index+by>=steps.length)return;const next=[...steps];[next[index],next[index+by]]=[next[index+by],next[index]];setSteps(next);setActive(index+by);};
  const removeStep=()=>{if(steps.length<=1)return;const next=steps.filter((_,index)=>index!==active);setSteps(next);setActive(Math.min(active,next.length-1));setError(null);};
  const save=()=>{if(!ready)return;try{const action=complete.length===1?{kind:"chord" as const,stroke:complete[0]!}:{kind:"sequence" as const,steps:complete.map(step=>step!)};onSave({id,on:entry?.on??true,label:label.trim(),...action});}catch{setError(gui.key_row_invalid);}};
  const group=(key:string)=>({letters:gui.key_row_letters,symbols:gui.keys_group_symbol,function:gui.key_row_function,navigation:gui.nav})[keyboardGroup(key)];
  return <form id={formId} data-key-entry-editor onSubmit={event=>{event.preventDefault();save();}} className="flex min-w-0 flex-col gap-row">
    <div data-key-entry-result data-key-entry-preview className="sticky top-0 z-10 flex min-w-0 flex-col gap-in bg-background py-in">
      <div className="flex min-w-0 items-center gap-in">
        <div ref={stepStrip} className="flex min-w-0 flex-1 items-center gap-in overflow-x-auto" data-key-entry-steps role="group" aria-label={gui.key_row_steps}>
          {steps.map((_step,index)=><div key={index} data-key-entry-step={index} className="flex min-w-0 flex-none items-center gap-in">
            {index>0&&<span aria-hidden className="text-muted-foreground">→</span>}
            <button type="button" data-key-step-select={index} aria-label={`${gui.key_row_step(index+1)} · ${complete[index]?strokeCaption(complete[index]!):gui.key_row_choose}`} aria-pressed={active===index}
              className={`flex min-h-touch flex-none items-center gap-in rounded-sm px-in text-body ${active===index?"bg-fill-strong font-title":"bg-fill"}`} onClick={()=>setActive(index)}>
              {steps.length>1&&<span aria-hidden className="text-caption text-muted-foreground tabular-nums">{index+1}</span>}
              <span>{complete[index]?strokeCaption(complete[index]!):gui.key_row_choose}</span>
            </button>
          </div>)}
        </div>

        {steps.length>1&&<DropdownMenu modal={false}>
          <DropdownMenuTrigger asChild><button type="button" data-key-step-menu aria-label={gui.key_row_step_actions(active+1)} className="grid min-h-touch min-w-touch flex-none place-items-center rounded-sm bg-fill"><IconMore className="size-affordance"/></button></DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuLabel>{gui.key_row_step(active+1)}</DropdownMenuLabel>
            <DropdownMenuItem data-key-step-up disabled={active===0} onSelect={()=>move(active,-1)}>{gui.key_row_earlier}</DropdownMenuItem>
            <DropdownMenuItem data-key-step-down disabled={active===steps.length-1} onSelect={()=>move(active,1)}>{gui.key_row_later}</DropdownMenuItem>
            <DropdownMenuSeparator/>
            <DropdownMenuItem data-key-step-remove className="text-destructive focus:text-destructive" onSelect={removeStep}>{gui.key_row_remove_step}</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>}
        <button type="button" className="grid min-h-touch min-w-touch flex-none place-items-center rounded-sm bg-fill disabled:opacity-40" data-key-step-add aria-label={gui.key_row_add_step} title={gui.key_row_add_step}
          disabled={!allComplete||steps.length>=KEY_ROW_LIMITS.steps} onClick={()=>{setSteps([...steps,emptyKeyboardStroke()]);setActive(steps.length);setKeyboard(true);setError(null);}}><IconPlus className="size-affordance"/></button>
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
