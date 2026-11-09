import {useRef} from "react";
import Keyboard from "react-simple-keyboard";
import type {KeyStroke} from "@/lib/term-key-model";
import {chooseKeyboardKey,keyboardLabels,keyboardPlatform,keyboardRows,modifierKeys,systemKeys} from "@/lib/term-keyboard-layout";
import {gui} from "@/gen/catalog";

/** Upstream renderer, Khor styling and terminal-safe keys. Never a session input. */
export function TerminalKeyboard({stroke,onChange}: {stroke:KeyStroke;onChange:(stroke:KeyStroke)=>void}) {
  const platform=keyboardPlatform(navigator.platform);
  const root=useRef<HTMLDivElement>(null),focused=useRef<string|null>(null);
  const labels=keyboardLabels(platform,stroke.shift),rows=keyboardRows(platform);
  const selected=[`{${stroke.key}}`,...Object.entries(modifierKeys).filter(([,modifier])=>stroke[modifier]).map(([key])=>`{${key}}`)].join(" ");
  const restoreFocus=()=>{
    if(focused.current && (document.activeElement===document.body || root.current?.contains(document.activeElement))){
      const key=[...root.current!.querySelectorAll<HTMLButtonElement>('[data-skbtn]')].find(el=>el.dataset.skbtn===focused.current);
      key?.focus({preventScroll:true});
    }
  };
  const pick=(key:string)=>onChange(chooseKeyboardKey(stroke,key));
  return <div data-terminal-keyboard data-keyboard-platform={platform} ref={root} className="terminal-keyboard-scroll" role="group" aria-label={gui.key_row_base_key}
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
  </div>;
}
