// Khor choice menus share the app's typography, glass, viewport and focus rules.
import {useId,useRef,type ReactNode} from "react";
import {IconChevronDown,IconChevronUp} from "@/components/icons";
import {DropdownMenu,DropdownMenuTrigger,DropdownMenuContent,DropdownMenuRadioGroup,DropdownMenuRadioItem,DropdownMenuCheckboxItem,DropdownMenuLabel} from "@/components/ui/dropdown-menu";
import {cn} from "@/lib/utils";

export type PopupChoice={
  value:string;label:string;short?:string;disabled?:boolean;group?:string;note?:string;
  preview?:ReactNode;
  mark?:Record<string,string|boolean|undefined>;
};
type Props={choices:PopupChoice[];label:string;disabled?:boolean;look?:"plain"|"field";
  mark?:Record<string,string|boolean|undefined>;className?:string;placeholder?:string};

function ChoicePopup({values,onPick,multiple=false,choices,label,disabled=false,look="plain",mark,className,placeholder}:Props&{values:string[];onPick:(value:string)=>void;multiple?:boolean}) {
  const id=useId(),menu=useRef<HTMLDivElement>(null);
  const selected=choices.filter(choice=>values.includes(choice.value));
  const current=selected.map(choice=>choice.short??choice.label).join(", ")||placeholder||values.join(", ");
  const runs:{group:string|undefined;items:PopupChoice[]}[]=[];
  for(const choice of choices){const last=runs.at(-1);if(last&&last.group===choice.group)last.items.push(choice);else runs.push({group:choice.group,items:[choice]});}
  const body=(choice:PopupChoice)=><>{choice.preview&&<span aria-hidden className="grid size-5 flex-none place-items-center">{choice.preview}</span>}<span className="min-w-0 flex-1"><span className="block break-words">{choice.label}</span>{choice.note&&<span className="block break-words text-callout text-muted-foreground">{choice.note}</span>}</span></>;
  return <DropdownMenu modal={false}>
    <div data-popup-button={look} className={cn("relative flex h-ctl-md min-w-0 items-center",look==="field"?"w-full":"flex-none",className)}>
      <DropdownMenuTrigger asChild><button type="button" value={values[0]??""} disabled={disabled} aria-label={`${label}: ${current}`} title={selected[0]?.note}
        data-popup-trigger data-popup-owner={id} {...mark}
        className={cn("flex min-h-touch min-w-0 items-center rounded-sm text-body outline-none focus-visible:outline-2 focus-visible:outline-ring disabled:opacity-40",look==="field"&&"w-full")}>
        <span className={cn("flex h-ctl-md min-w-0 items-center gap-in rounded-sm pr-xxs pl-in hover:bg-fill",look==="field"&&"w-full bg-fill hover:bg-fill-strong")}>
          {!multiple&&selected[0]?.preview&&<span aria-hidden className="grid size-5 flex-none place-items-center">{selected[0].preview}</span>}
          <span data-popup-value className={cn("min-w-0 truncate tabular-nums",look==="field"&&"flex-1")}>{current}</span>
          <span data-setting-trail className="flex h-4.5 w-4 flex-none flex-col items-center justify-center rounded-xs bg-fill-strong text-muted-foreground"><IconChevronUp centred className="-mb-(--chevron-pair) size-2.5"/><IconChevronDown centred className="size-2.5"/></span>
        </span>
      </button></DropdownMenuTrigger>
    </div>
    <DropdownMenuContent ref={menu} data-popup-menu={id} align="end" aria-label={label}
      className="min-w-(--radix-dropdown-menu-trigger-width) max-w-[calc(100vw-2*var(--gap-in))] font-sans"
      onEntryFocus={event=>{event.preventDefault();requestAnimationFrame(()=>{const chosen=menu.current?.querySelector<HTMLElement>('[data-on="true"]:not([data-disabled])')??menu.current?.querySelector<HTMLElement>('[data-popup-option]:not([data-disabled])');chosen?.focus({preventScroll:true});});}}>
      <DropdownMenuRadioGroup value={values[0]??""} onValueChange={multiple?undefined:onPick}>
        {runs.map((run,index)=><div key={index}>{run.group&&<DropdownMenuLabel>{run.group}</DropdownMenuLabel>}{run.items.map(choice=>multiple?
          <DropdownMenuCheckboxItem key={choice.value} checked={values.includes(choice.value)} disabled={choice.disabled} textValue={choice.label} data-popup-option={choice.value} data-on={values.includes(choice.value)} {...choice.mark} onSelect={event=>event.preventDefault()} onCheckedChange={()=>onPick(choice.value)}>{body(choice)}</DropdownMenuCheckboxItem>:
          <DropdownMenuRadioItem key={choice.value} indicator="check" value={choice.value} disabled={choice.disabled} textValue={choice.label} data-popup-option={choice.value} data-on={values.includes(choice.value)} {...choice.mark}>{body(choice)}</DropdownMenuRadioItem>)}</div>)}
      </DropdownMenuRadioGroup>
    </DropdownMenuContent>
  </DropdownMenu>;
}
export function PopupButton({value,onPick,...props}:Props&{value:string;onPick:(value:string)=>void}) {
  return <ChoicePopup {...props} values={[value]} onPick={onPick}/>;
}
export function MultiPopupButton({value,onPick,...props}:Props&{value:string[];onPick:(value:string[])=>void}) {
  return <ChoicePopup {...props} multiple values={value} onPick={key=>onPick(value.includes(key)?value.filter(item=>item!==key):[...value,key])}/>;
}
