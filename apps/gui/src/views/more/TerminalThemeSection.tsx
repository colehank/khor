import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { inApp, saveTerminalPalette } from "@/api";
import { gui } from "@/gen/catalog";
import { MoreSection } from "@/views/more/MorePage";
import { PopupButton } from "@/components/ui/popup-button";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Dialog, DialogContent, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { ANSI_KEYS, BASE_KEYS, PRESETS, colorHex, clonePalette, importPalette, onTermTheme, readPalette, removePalette, savePalette, setTermTheme, termTheme, terminalTheme, useTermTheme, THEME_FILE_MAX, type Palette, type ColorKey } from "@/lib/term-theme";
import { useWindowSupport } from "@/lib/window-surface";

const SAMPLE = "\x1b[32mzgh@MacLab\x1b[0m \x1b[34m~/khor\x1b[0m\r\n中文 · English · \x1b[31merror\x1b[0m · \x1b[33mwarning\x1b[0m\r\n\x1b[30m0 \x1b[31m1 \x1b[32m2 \x1b[33m3 \x1b[34m4 \x1b[35m5 \x1b[36m6 \x1b[37m7\x1b[0m    \x1b[90m8 \x1b[91m9 \x1b[92m10 \x1b[93m11 \x1b[94m12 \x1b[95m13 \x1b[96m14 \x1b[97m15\x1b[0m\r\n\x1b[38;5;16m16\x1b[0m · \x1b[38;2;128;160;200mRGB\x1b[0m > ";

/** A real xterm preview; it has no host, input handler or live session. */
export function TerminalThemePreview({ theme }: { theme?: Palette }) {
  const box = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const themeRef = useRef(theme); themeRef.current = theme;
  const [ready, setReady] = useState(false);
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const css = getComputedStyle(el);
    const term = new Terminal({ cols: 58, rows: 4, disableStdin: true, scrollback: 0, cursorBlink: false, fontFamily: css.fontFamily, fontSize: parseFloat(css.fontSize), theme: theme ?? terminalTheme(el) });
    termRef.current = term;
    const fit = new FitAddon(); term.loadAddon(fit); term.open(el); term.write(SAMPLE);
    (el as HTMLDivElement & { __xterm?: Terminal }).__xterm = term;
    const paint = () => { term.options.theme = themeRef.current ?? terminalTheme(el); };
    const observer = new MutationObserver(paint);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    const stop = onTermTheme(paint);
    const resized = new ResizeObserver(() => { if (el.clientWidth) fit.fit(); }); resized.observe(el);
    setReady(true);
    return () => { observer.disconnect(); resized.disconnect(); stop(); term.dispose(); termRef.current = null; };
  }, []);
  useEffect(() => { if (ready && termRef.current && box.current) termRef.current.options.theme = theme ?? terminalTheme(box.current); }, [ready, theme]);
  return <div ref={box} data-term-theme-preview className="overflow-hidden rounded-md p-in font-term text-term text-foreground" style={{ backgroundColor: theme?.background ?? "var(--term-bg)", color: theme?.foreground ?? "var(--term-fg)", height: "var(--theme-preview-h)", fontFamily: "var(--font-term)", fontSize: "var(--text-term)" }} />;
}

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
  const [removing, setRemoving] = useState(false);
  const custom = choice.customs.find(p => p.id === choice.selected);
  const names = [{ value: "follow", label: gui.term_theme_follow }, ...PRESETS.map(p => ({ value: p.id, label: ({ soft: gui.term_theme_soft, warm: gui.term_theme_warm, contrast: gui.term_theme_contrast } as Record<string, string>)[p.id] ?? p.name, group: gui.term_theme_presets })), ...choice.customs.map(p => ({ value: p.id, label: p.name, group: gui.term_theme_mine }))];
  const selectedName = names.find(n => n.value === choice.selected)?.label ?? gui.term_theme_follow;
  const clone = () => clonePalette(sectionRef.current ?? document.body);
  const edit = () => { setError(null); const theme = clone(); lastPreview.current = theme; setEditor({ name: custom?.name ?? gui.term_theme_copy_name(selectedName), theme, id: custom?.id }); setExtra(theme.extendedAnsi?.join(" ") ?? ""); };
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
  return <div ref={sectionRef}>
    <MoreSection name="term-theme" label={gui.term_theme_title}>
      <div data-setting-row="popup" data-setting-row-name="term-theme" className="flex items-center gap-row px-row py-in">
        <span className="min-w-0 flex-1 text-body">{gui.term_theme_palette}</span>
        <PopupButton label={gui.term_theme_palette} value={choice.selected} choices={names} onPick={selected => { const stored = setTermTheme({ ...choice, selected }); setNotice(null); setError(stored ? null : gui.term_theme_storage_error); }} mark={{ "data-setting-select": "term-theme" }} className="-mr-xxs" />
      </div>
      <div className="px-row pb-in"><TerminalThemePreview /></div>
      <div className="flex flex-wrap items-center justify-end gap-in px-row pb-in">
        <Button data-term-theme-edit onClick={edit}>{custom ? gui.edit_ask : gui.term_theme_customize}</Button>
        <Button data-term-theme-import onClick={() => fileRef.current?.click()}>{gui.import_sessions_ask}</Button>
        <Button data-term-theme-export onClick={() => void exportFile()}>{gui.term_theme_export}</Button>
        {custom && <Button data-term-theme-remove onClick={() => setRemoving(true)} className="text-destructive">{gui.remove}</Button>}
      </div>
      <input ref={fileRef} data-term-theme-file type="file" accept=".json,application/json" className="hidden" onChange={e => { void importFile(e.target.files?.[0]); e.target.value = ""; }} />
    </MoreSection>
    {(error || notice) && <p role="status" className={`mb-group text-callout ${error ? "text-destructive" : "text-muted-foreground"}`}>{error ?? notice}</p>}
    <Dialog open={editor !== null} onOpenChange={open => { if (!open) setEditor(null); }}>
      <DialogContent form data-term-theme-editor className="term-theme-editor w-(--theme-editor-w)! overflow-y-auto">
        <DialogTitle>{gui.term_theme_customize_title}</DialogTitle>
        {editor && <>
          <label className="flex items-center gap-row text-body">{gui.new_session_name}<Input aria-label={gui.new_session_name} value={editor.name} maxLength={48} onChange={e => setEditor({ ...editor, name: e.target.value })} /></label>
          <TerminalThemePreview theme={valid ?? lastPreview.current ?? undefined} />
          <h3 className="text-callout text-muted-foreground">{gui.term_theme_basic}</h3>
          <div className="term-theme-grid">{BASE_KEYS.map(k => <ColorField key={k} name={k} value={editor.theme[k]} onChange={v => setEditor({ ...editor, theme: { ...editor.theme, [k]: v } })} />)}</div>
          <h3 className="text-callout text-muted-foreground">{gui.term_theme_ansi}</h3>
          <div className="term-theme-grid">{Array.from({ length: 8 }, (_, i) => [ANSI_KEYS[i], ANSI_KEYS[i + 8]]).flat().map(k => <ColorField key={k} name={k} value={editor.theme[k]} onChange={v => setEditor({ ...editor, theme: { ...editor.theme, [k]: v } })} />)}</div>
          <details className="text-callout"><summary>{gui.term_theme_extended}</summary><p className="my-in text-muted-foreground">{gui.term_theme_extended_note}</p><textarea data-term-theme-extended aria-label={gui.term_theme_extended} value={extra} onChange={e => setExtra(e.target.value)} spellCheck={false} className="h-24 w-full rounded-sm border bg-fill p-in font-mono text-callout" /></details>
          {(invalid || error) && <p role="status" className="text-callout text-destructive">{invalid ?? error}</p>}
          <DialogFooter><Button onClick={() => setEditor(null)}>{gui.delete_keep}</Button><Button data-term-theme-save variant="primary" disabled={!valid || !editor.name.trim()} onClick={() => { if (!valid) return; try { savePalette(editor.name, valid, editor.id); setEditor(null); setNotice(gui.term_theme_saved); } catch (e) { if (e instanceof Error && e.message === "storage") setEditor({ ...editor, id: termTheme().selected }); setError(colorError(e)); } }}>{gui.term_theme_save}</Button></DialogFooter>
        </>}
      </DialogContent>
    </Dialog>
    <Dialog open={removing} onOpenChange={setRemoving}><DialogContent><DialogTitle>{gui.term_theme_remove_name(custom?.name ?? "")}</DialogTitle><DialogFooter><Button onClick={() => setRemoving(false)}>{gui.delete_keep}</Button><Button variant="destructive" onClick={() => { if (custom && !removePalette(custom.id)) setError(gui.term_theme_storage_error); setRemoving(false); }}>{gui.remove}</Button></DialogFooter></DialogContent></Dialog>
  </div>;
}
