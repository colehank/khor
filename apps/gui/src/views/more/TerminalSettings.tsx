import {useEffect,useState} from "react";
import {IconChevronRight} from "@/components/icons";
import {Slider} from "@/components/settings/Slider";
import {PopupRow} from "@/components/settings/PopupRow";
import {SwitchRow} from "@/components/settings/SwitchRow";
import {Button} from "@/components/ui/button";
import {Input} from "@/components/ui/input";
import {gui} from "@/gen/catalog";
import {useCoarse} from "@/hooks/use-coarse";
import {setTermCursor,useTermCursor,type CursorShape} from "@/lib/term-cursor";
import {HISTORY_CHOICES,setTermHistory,useTermHistory} from "@/lib/term-history";
import {setTermBell,useTermBell} from "@/lib/term-bell";
import {setTermBlockLines,useTermBlockLines} from "@/lib/term-block-lines";
import {setTermPredict,useTermPredict} from "@/lib/term-predict-setting";
import {setTermOptionMeta,useTermOptionMeta} from "@/lib/term-option-meta";
import {TERM_MAX,TERM_MIN,setTypeSize,useTypeSize} from "@/lib/type-size";
import {clonePalette,onTermTheme} from "@/lib/term-theme";
import {KeyRowEditor} from "@/views/TermKeys";
import {MoreSection} from "./MorePage";
import {TerminalFontRow} from "./TerminalFontRow";
import {TerminalReadabilityRow,TerminalThemeSection} from "./TerminalThemeSection";
import {TerminalThemePreview} from "./TerminalThemePreview";

const ROW="flex items-center gap-row px-row py-in";

function TerminalSizeRow() {
  const size=useTypeSize();
  const [draft,setDraft]=useState(String(size.term));
  useEffect(()=>setDraft(String(size.term)),[size.term]);
  const commit=()=>{
    const value=Number(draft);
    if(!draft.trim()||!Number.isFinite(value)){setDraft(String(size.term));return;}
    const term=Math.max(TERM_MIN,Math.min(TERM_MAX,Math.round(value)));
    setDraft(String(term));setTypeSize({...size,term});
  };
  return <div data-setting-row="slider" data-setting-row-name="term-size" className="flex flex-col gap-in px-row py-in">
    <div className="flex items-center gap-row">
      <label htmlFor="terminal-size" className="min-w-0 flex-1 text-body">{gui.type_size}</label>
      <Input id="terminal-size" data-term-size-number type="number" inputMode="numeric" min={TERM_MIN} max={TERM_MAX} step={1}
        value={draft} onChange={event=>{
          const next=event.target.value;setDraft(next);
          const term=Number(next);
          if(next.trim()&&Number.isInteger(term)&&term>=TERM_MIN&&term<=TERM_MAX)setTypeSize({...size,term});
        }} onBlur={commit} onKeyDown={event=>{if(event.key==="Enter"){event.preventDefault();commit();}}}
        className="w-16 min-w-0 text-right tabular-nums"/>
    </div>
    <Slider label={gui.type_size} min={TERM_MIN} max={TERM_MAX} value={size.term}
      ticks={(TERM_MAX-TERM_MIN)/2+1} onChange={term=>setTypeSize({...size,term})}
      small={<span className="text-caption">A</span>} large={<span className="text-title-sm">A</span>}/>
  </div>;
}

function CursorColorRow() {
  const cursor=useTermCursor();
  const [paletteColor,setPaletteColor]=useState(()=>clonePalette(document.body).cursor);
  useEffect(()=>onTermTheme(()=>setPaletteColor(clonePalette(document.body).cursor)),[]);
  return <div data-setting-row="value" data-setting-row-name="term-cursor-color" className={ROW}>
    <span className="min-w-0 flex-1 text-body">{gui.term_cursor_color}</span>
    {cursor.color ? <Button variant="ghost" size="sm" data-term-cursor-auto onClick={()=>setTermCursor({...cursor,color:null})}>{gui.term_color_auto}</Button>
      : <span className="text-callout text-muted-foreground">{gui.term_color_auto}</span>}
    <input type="color" data-term-cursor-color aria-label={gui.term_cursor_color} value={cursor.color??paletteColor}
      onChange={event=>setTermCursor({...cursor,color:event.target.value})} className="term-theme-well"/>
  </div>;
}

/** Appearance and behaviour remain separate; both configure the live terminal. */
export function TerminalSection() {
  const coarse=useCoarse();
  const cursor=useTermCursor(),history=useTermHistory(),bell=useTermBell();
  const lines=useTermBlockLines(),predict=useTermPredict(),optionMeta=useTermOptionMeta();
  const [editingKeys,setEditingKeys]=useState(false);
  const apple=/Mac|iPhone|iPad|iPod/.test(navigator.platform);
  return <>
    <MoreSection name="terminal-appearance" label={gui.term_appearance}>
      <TerminalThemeSection/>
      <TerminalFontRow/>
      <TerminalSizeRow/>
      <TerminalReadabilityRow/>
      <PopupRow name="term-cursor" label={gui.term_cursor} value={cursor.shape} choices={[
        {value:"block",label:gui.term_cursor_block},{value:"bar",label:gui.term_cursor_bar},{value:"underline",label:gui.term_cursor_underline},
      ]} onPick={shape=>setTermCursor({...cursor,shape:shape as CursorShape})}/>
      <SwitchRow name="term-cursor-blink" label={gui.term_cursor_blink} on={cursor.blink} onToggle={()=>setTermCursor({...cursor,blink:!cursor.blink})}/>
      <CursorColorRow/>
      <SwitchRow name="term-bell" label={gui.term_bell} on={bell} onToggle={()=>setTermBell(!bell)}/>
      <SwitchRow name="term-block-lines" label={gui.term_block_lines} on={lines} onToggle={()=>setTermBlockLines(!lines)}/>
      <div data-terminal-appearance-preview className="px-row pb-row pt-in"><TerminalThemePreview/></div>
    </MoreSection>
    <MoreSection name="terminal-functions" label={gui.term_functions}>
      <button type="button" data-setting-row="nav" data-setting-row-name="term-key-row" onClick={()=>setEditingKeys(true)} className={`${ROW} w-full text-start`}>
        <span className="min-w-0 flex-1 text-body">{gui.term_key_row}</span><IconChevronRight className="size-affordance flex-none text-tertiary"/>
      </button>
      <PopupRow name="term-history" label={gui.term_history} value={String(history)} choices={HISTORY_CHOICES.map(n=>({value:String(n),label:n.toLocaleString()}))}
        onPick={n=>setTermHistory(Number(n))}/>
      <SwitchRow name="term-predict" label={gui.term_predict} on={predict} onToggle={()=>setTermPredict(!predict)}/>
      {!coarse&&apple&&<SwitchRow name="term-option-meta" label={gui.term_option_meta} on={optionMeta} onToggle={()=>setTermOptionMeta(!optionMeta)}/>}
    </MoreSection>
    <KeyRowEditor open={editingKeys} onOpenChange={setEditingKeys}/>
  </>;
}
