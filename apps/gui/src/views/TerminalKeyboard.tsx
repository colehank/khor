import {useEffect,useRef,useState} from "react";
import Keyboard from "react-simple-keyboard";
import {BASE_KEYS} from "@/lib/term-key-model";
import {useNarrow} from "@/hooks/use-narrow";
import {SegmentedControl} from "@/components/ui/segmented-control";
import {chooseKeyboardKey,keyboardLabels,keyboardPlatform,keyboardRows,modifierKeys,systemKeys,keyboardGroup,type KeyboardStroke} from "@/lib/term-keyboard-layout";
import {gui} from "@/gen/catalog";

/** Upstream renderer, Khor styling and terminal-safe keys. Never a session input. */
export function TerminalKeyboard({stroke,onChange}: {stroke:KeyboardStroke;onChange:(stroke:KeyboardStroke)=>void}) {
  const platform=keyboardPlatform(navigator.platform);
  const root=useRef<HTMLDivElement>(null),focused=useRef<string|null>(null);
  const narrow=useNarrow();
  const [page,setPage]=useState(keyboardGroup(stroke.key));
  useEffect(()=>{if(stroke.key)setPage(keyboardGroup(stroke.key));},[stroke.key]);
  const labels=keyboardLabels(platform,stroke.shift);
  const codes=keyboardRows(platform).join(" ").split(" ").map(key=>key.slice(1,-1)).filter(key=>BASE_KEYS.includes(key as typeof BASE_KEYS[number])&&keyboardGroup(key)===page);
  const compact=Array.from({length:Math.ceil(codes.length/5)},(_,row)=>codes.slice(row*5,row*5+5).map(key=>`{${key}}`).join(" "));
  const rows=narrow?compact:keyboardRows(platform);
  const selected=[...(stroke.key?[`{${stroke.key}}`]:[]),...Object.entries(modifierKeys).filter(([,modifier])=>stroke[modifier]).map(([key])=>`{${key}}`)].join(" ");
  const restoreFocus=()=>{
    if(focused.current && (document.activeElement===document.body || root.current?.contains(document.activeElement))){
      const key=[...root.current!.querySelectorAll<HTMLButtonElement>('[data-skbtn]')].find(el=>el.dataset.skbtn===focused.current);
      key?.focus({preventScroll:true});
    }
  };
  const pick=(key:string)=>onChange(chooseKeyboardKey(stroke,key));
  return <div className="flex min-w-0 flex-col gap-in">
    {narrow&&<SegmentedControl label={gui.keyboard_show} value={page} options={[
      {key:"navigation",label:gui.nav},{key:"letters",label:gui.key_row_letters},
      {key:"symbols",label:gui.keys_group_symbol},{key:"function",label:gui.key_row_function},
    ]} onChange={setPage}/>}
    <div data-terminal-keyboard data-keyboard-compact={narrow||undefined} data-keyboard-platform={platform} ref={root} className="terminal-keyboard-scroll" role="group" aria-label={gui.key_row_base_key}
    onFocusCapture={event=>{focused.current=(event.target as HTMLElement).closest<HTMLButtonElement>('[data-skbtn]')?.dataset.skbtn??null;}}
    onBlurCapture={event=>{if(event.relatedTarget&&!event.currentTarget.contains(event.relatedTarget))focused.current=null;}}
    onKeyDownCapture={event=>{
      if((event.key==="Enter"||event.key===" ")&&!event.repeat){
        const button=(event.target as HTMLElement).closest<HTMLButtonElement>('[data-skbtn]');
        if(button&&!button.disabled){event.preventDefault();event.stopPropagation();pick(button.dataset.skbtn!);}
      }
    }}>
    <Keyboard theme="khor-keyboard" layout={{default:rows}} display={labels} useButtonTag disableButtonHold
      physicalKeyboardHighlight={false} syncInstanceInputs={false} maxLength={0} onKeyPress={pick} onRender={restoreFocus}
      buttonTheme={[{class:"key-selected",buttons:selected}]}
      buttonAttributes={[
        {attribute:"type",value:"button"},
        {attribute:"aria-pressed",value:"false"},
        {attribute:"aria-pressed",value:"true",buttons:selected},
        ...Object.entries(labels).map(([buttons,value])=>({attribute:"aria-label",value,buttons})),
        {attribute:"disabled",value:"true",buttons:systemKeys.map(key=>`{${key}}`).join(" ")},
        {attribute:"title",value:gui.key_row_system_key,buttons:systemKeys.map(key=>`{${key}}`).join(" ")},
      ]}/>
  </div></div>;
}
