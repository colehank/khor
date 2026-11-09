import {SectionHeading} from "@/components/settings/SectionHeading";
import { useEffect, useRef, useState } from "react";
import { inApp, saveTerminalPalette } from "@/api";
import { config, gui } from "@/gen/catalog";
import { PopupButton } from "@/components/ui/popup-button";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Dialog, DialogContent, TaskOverlay, DialogBody, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { ANSI_KEYS, BASE_KEYS, PRESETS, THEME_FAMILIES, DEFAULT_CONTRAST, colorHex, clonePalette, importPalette, readPalette, removePalette, savePalette, setTermTheme, termTheme, useTermTheme, THEME_FILE_MAX, type Palette, type ColorKey } from "@/lib/term-theme";
import { useWindowSupport } from "@/lib/window-surface";

import {TerminalThemePreview} from "./TerminalThemePreview";
export {TerminalThemePreview} from "./TerminalThemePreview";

const labels = (): Record<ColorKey, string> => ({
  background: gui.term_color_background, foreground: gui.term_color_foreground, cursor: gui.key_cursor, cursorAccent: gui.term_color_cursor_accent,
  selectionBackground: gui.term_color_selection, selectionForeground: gui.term_color_selection_foreground,
  black: gui.term_color_black, red: gui.term_color_red, green: gui.term_color_green, yellow: gui.term_color_yellow, blue: gui.term_color_blue, magenta: gui.term_color_magenta, cyan: gui.term_color_cyan, white: gui.term_color_white,
  brightBlack: gui.term_color_bright(gui.term_color_black), brightRed: gui.term_color_bright(gui.term_color_red), brightGreen: gui.term_color_bright(gui.term_color_green), brightYellow: gui.term_color_bright(gui.term_color_yellow), brightBlue: gui.term_color_bright(gui.term_color_blue), brightMagenta: gui.term_color_bright(gui.term_color_magenta), brightCyan: gui.term_color_bright(gui.term_color_cyan), brightWhite: gui.term_color_bright(gui.term_color_white),
});
function colorError(e: unknown): string {
  const key = e instanceof Error ? e.message : "colors";
  return key === "storage" ? gui.term_theme_storage_error : key === "json" ? gui.term_theme_bad_json : key === "large" ? gui.term_theme_large : key === "extended" ? gui.term_theme_bad_extended : key === "full" ? gui.term_theme_full : gui.term_theme_bad_color;
}

function ColorField({ name, value, onChange }: { name: ColorKey; value: string; onChange: (v: string) => void }) {
  const label = labels()[name];
  const alpha = name === "selectionBackground" || name === "selectionForeground";
  const hex = colorHex(value, alpha);
  const previous = useRef(value);
  if (hex) previous.current = hex;
  const shown = hex ?? colorHex(previous.current, alpha) ?? PRESETS[0].theme[name];
  return <div data-theme-color={name} className="flex min-w-0 items-center gap-in rounded-sm bg-fill p-in">
    <label htmlFor={`theme-${name}`} className="min-w-0 flex-1 text-callout">{label}</label>
    <input type="color" aria-label={gui.term_color_pick(label)} value={shown.slice(0, 7)} onChange={e => onChange(e.target.value + (alpha && shown.length === 9 ? shown.slice(7) : ""))} className="term-theme-well" />
    <Input id={`theme-${name}`} aria-label={gui.term_color_hex(label)} aria-invalid={!hex} value={value} onChange={e => onChange(e.target.value)} spellCheck={false} maxLength={alpha ? 9 : 7} className="term-theme-hex h-ctl-md font-mono text-callout tabular-nums" />
  </div>;
}

export function TerminalThemeSection() {
  const choice = useTermTheme();
  const support = useWindowSupport();
  const fileRef = useRef<HTMLInputElement>(null);
  const sectionRef = useRef<HTMLDivElement>(null);
  const [editor, setEditor] = useState<{ name: string; theme: Palette; id?: string } | null>(null);
  const lastPreview = useRef<Palette | null>(null);
  const [extra, setExtra] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [removing, setRemoving] = useState<{id:string;name:string}|null>(null);
  const custom = choice.customs.find(p => p.id === choice.selected);
  const [making,setMaking]=useState(false);
  useEffect(()=>setMaking(false),[choice.selected]);
  const names = [{ value: "follow", label: gui.term_theme_follow },{value:"custom",label:gui.term_theme_customize}, ...THEME_FAMILIES.map(p => ({ value: p.id, label: p.name, group: gui.term_theme_automatic })), ...PRESETS.map(p => ({ value: p.id, label: ({ soft: gui.term_theme_soft, warm: gui.term_theme_warm, contrast: gui.term_theme_contrast } as Record<string, string>)[p.id] ?? p.name, group: gui.term_theme_presets })), ...choice.customs.map(p => ({ value: p.id, label: p.name, group: gui.term_theme_mine }))];
  const selectedName = names.find(n => n.value === choice.selected)?.label ?? gui.term_theme_follow;
  const clone = () => clonePalette(sectionRef.current ?? document.body);
  const edit = () => { setError(null); const theme = clone(); lastPreview.current = theme; setEditor({ name: (!making&&custom?.name) || gui.term_theme_copy_name(selectedName), theme, id: making?undefined:custom?.id }); setExtra(theme.extendedAnsi?.join(" ") ?? ""); };
  let valid: Palette | null = null;
  let invalid: string | null = null;
  if (editor) try {
    const extendedAnsi = extra.trim() ? extra.trim().split(/[\s,]+/) : undefined;
    const input = { ...editor.theme }; delete input.extendedAnsi;
    valid = readPalette(extendedAnsi ? { ...input, extendedAnsi } : input, PRESETS[0].theme);
    lastPreview.current = valid;
  } catch (e) { invalid = colorError(e); }
  const importFile = async (file?: File) => {
    if (!file) return;
    setError(null); setNotice(null);
    try {
      if (file.size > THEME_FILE_MAX) throw new Error("large");
      const p = importPalette(await file.text(), clone());
      savePalette(p.name || file.name.replace(/\.json$/i, ""), p.theme);
      setMaking(false);
      setNotice(gui.term_theme_imported);
    } catch (e) { setError(colorError(e)); }
  };
  const exportFile = async () => {
    setError(null); setNotice(null);
    const text = JSON.stringify({ name: selectedName, theme: clone() }, null, 2) + "\n";
    try {
      if (inApp && support.supported) {
        if (await saveTerminalPalette(text)) setNotice(gui.term_theme_exported);
      } else if (inApp) {
        await navigator.clipboard.writeText(text); setNotice(gui.term_theme_copied);
      } else {
        const href = URL.createObjectURL(new Blob([text], { type: "application/json" }));
        const a = document.createElement("a"); a.href = href; a.download = "khor-terminal-theme.json"; a.click();
        setTimeout(() => URL.revokeObjectURL(href), 1000); setNotice(gui.term_theme_exported);
      }
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
  };
  return <>
    <div ref={sectionRef} data-setting-row="popup" data-setting-row-name="term-theme" className="flex items-center gap-row px-row py-in">
      <span className="min-w-0 flex-1 text-body">{gui.term_theme_title}</span>
      <PopupButton label={gui.term_theme_title} value={making?"custom":choice.selected} choices={names}
        onPick={selected=>{
          setNotice(null);setError(null);
          if(selected==="custom"){setMaking(true);return;}
          setMaking(false);const stored=setTermTheme({...choice,selected});setError(stored?null:gui.term_theme_storage_error);
        }}
        mark={{"data-setting-select":"term-theme"}} className="-mr-xxs"/>
    </div>
    {(making || custom) && <div data-setting-row="action" data-term-theme-custom className="flex flex-wrap items-center gap-in px-row py-in">
      <span className="min-w-0 flex-1 text-body">{gui.term_theme_custom_row}</span>
      <Button data-term-theme-import onClick={()=>fileRef.current?.click()}>{gui.import_sessions}</Button>
      <Button data-term-theme-edit onClick={edit}>{gui.nav_settings}</Button>
    </div>}
    <input ref={fileRef} data-term-theme-file type="file" accept=".json,application/json" className="hidden" onChange={event=>{void importFile(event.target.files?.[0]);event.target.value="";}}/>
    {(error || notice) && <p role="status" className={`px-row py-in text-callout ${error?"text-destructive":"text-muted-foreground"}`}>{error??notice}</p>}
    <Dialog open={editor !== null} onOpenChange={open => { if (!open) setEditor(null); }}>
      <TaskOverlay purpose="edit" initialHeight="large" maxHeight="large" scrollable data-term-theme-editor className="term-theme-editor w-(--theme-editor-w)">
        <DialogTitle>{gui.term_theme_customize_title}</DialogTitle>
        {editor && <>
          <DialogBody>
          <label className="flex flex-none items-center gap-row text-body"><span className="flex-none whitespace-nowrap">{gui.new_session_name}</span><Input className="min-w-0 flex-1" aria-label={gui.new_session_name} value={editor.name} maxLength={48} onChange={e => setEditor({ ...editor, name: e.target.value })} /></label>
          <TerminalThemePreview theme={valid ?? lastPreview.current ?? undefined} />
          <SectionHeading className="px-in">{gui.term_theme_basic}</SectionHeading>
          <div className="term-theme-grid">{BASE_KEYS.map(k => <ColorField key={k} name={k} value={editor.theme[k]} onChange={v => setEditor({ ...editor, theme: { ...editor.theme, [k]: v } })} />)}</div>
          <SectionHeading className="px-in">{gui.term_theme_ansi}</SectionHeading>
          <div className="term-theme-grid">{Array.from({ length: 8 }, (_, i) => [ANSI_KEYS[i], ANSI_KEYS[i + 8]]).flat().map(k => <ColorField key={k} name={k} value={editor.theme[k]} onChange={v => setEditor({ ...editor, theme: { ...editor.theme, [k]: v } })} />)}</div>
          <details className="text-callout"><summary>{gui.term_theme_extended}</summary><p className="my-in text-muted-foreground">{gui.term_theme_extended_note}</p><textarea data-term-theme-extended aria-label={gui.term_theme_extended} value={extra} onChange={e => setExtra(e.target.value)} spellCheck={false} className="h-24 w-full rounded-sm border bg-fill p-in font-mono text-callout" /></details>
          {(invalid || error) && <p role="status" className="text-callout text-destructive">{invalid ?? error}</p>}
          </DialogBody>
          <DialogFooter><div className="mr-auto flex flex-wrap gap-in"><Button data-term-theme-export onClick={()=>void exportFile()}>{gui.term_theme_export}</Button>{editor.id && <Button data-term-theme-remove variant="ghost" onClick={()=>setRemoving({id:editor.id!,name:editor.name})}>{gui.remove}</Button>}</div><Button onClick={() => setEditor(null)}>{gui.delete_keep}</Button><Button data-term-theme-save variant="primary" disabled={!valid || !editor.name.trim()} onClick={() => { if (!valid) return; try { savePalette(editor.name, valid, editor.id); setMaking(false); setEditor(null); setNotice(gui.term_theme_saved); } catch (e) { if (e instanceof Error && e.message === "storage") setEditor({ ...editor, id: termTheme().selected }); setError(colorError(e)); } }}>{gui.term_theme_save}</Button></DialogFooter>
        </>}
      </TaskOverlay>
    </Dialog>
    <Dialog open={removing!==null} onOpenChange={open=>{if(!open)setRemoving(null);}}><DialogContent><DialogTitle>{gui.term_theme_remove_name(removing?.name ?? "")}</DialogTitle><DialogFooter><Button onClick={() => setRemoving(null)}>{gui.delete_keep}</Button><Button variant="destructive" onClick={() => { if (removing && !removePalette(removing.id)) setError(gui.term_theme_storage_error);else setEditor(null); setRemoving(null); }}>{gui.remove}</Button></DialogFooter></DialogContent></Dialog>
  </>;
}


export function TerminalReadabilityRow() {
  const choice=useTermTheme();
  const [error,setError]=useState(false);
  return <>
    <div data-setting-row="popup" data-setting-row-name="term-contrast" className="flex items-center gap-row px-row py-in">
      <span className="min-w-0 flex-1 text-body">{gui.term_theme_readability}</span>
      <PopupButton label={gui.term_theme_readability} value={String(choice.contrast??DEFAULT_CONTRAST)} choices={[
        {value:"1",label:config.off},{value:"3",label:gui.term_theme_protect_light},
        {value:"4.5",label:gui.term_theme_protect_standard},{value:"7",label:gui.term_theme_protect_strong},
      ]} onPick={value=>setError(!setTermTheme({...choice,contrast:Number(value)}))}
        mark={{"data-setting-select":"term-contrast"}} className="-mr-xxs"/>
    </div>
    {error && <p role="alert" className="px-row py-in text-callout text-destructive">{gui.term_theme_storage_error}</p>}
  </>;
}
