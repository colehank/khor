import { useState } from "react";

import { IconChevronDown, IconChevronUp, IconClose } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { gui } from "@/gen/catalog";
import { actionCaption, strokeCaption } from "@/lib/term-key-codec";
import { KEY_ROW_LIMITS, type CustomKeyEntry, type KeyStroke } from "@/lib/term-key-model";

import { TerminalKeyboard } from "@/views/TerminalKeyboard";

const emptyStroke = (): KeyStroke => ({ key: "ArrowLeft", ctrl: false, alt: false, shift: true });


export function KeyRowEntryEditor({ entry, onSave, onCancel, onDelete }: {
  entry: CustomKeyEntry | null;
  onSave: (entry: CustomKeyEntry) => void;
  onCancel: () => void;
  onDelete?: () => void;
}) {
  const [id] = useState(() => entry?.id ?? `custom:${crypto.randomUUID()}`);
  const [label, setLabel] = useState(entry?.label ?? "");
  const [kind, setKind] = useState<"chord" | "sequence">(entry?.kind ?? "chord");
  const [steps, setSteps] = useState<KeyStroke[]>(() => entry ? entry.kind === "chord" ? [{ ...entry.stroke }] : entry.steps.map(step => ({ ...step })) : [emptyStroke()]);
  const [active, setActive] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const action = kind === "chord" ? { kind, stroke: steps[0] } as const : { kind, steps } as const;
  const caption = actionCaption(action);
  const update = (next: KeyStroke[]) => { setSteps(next); setError(null); };
  const move = (at: number, by: number) => {
    const next = [...steps];
    [next[at], next[at + by]] = [next[at + by], next[at]];
    update(next);
    setActive(at+by);
  };
  const save = () => {
    const name = label.trim();
    if ([...name].length > KEY_ROW_LIMITS.labelCharacters || new TextEncoder().encode(name).length > KEY_ROW_LIMITS.labelBytes) {
      setError(gui.key_row_invalid);
      return;
    }
    try { onSave({ id, on: entry?.on ?? true, label: name, ...action }); }
    catch { setError(gui.key_row_invalid); }
  };
  return (
    <section data-key-entry-editor className="flex min-w-0 flex-col gap-row py-in">
      <h3 className="text-body font-label">{entry ? gui.key_row_edit : gui.key_row_add}</h3>
      <label className="flex min-w-0 flex-col gap-in text-body">
        {gui.key_row_name}
        <Input autoFocus data-key-entry-name value={label} maxLength={KEY_ROW_LIMITS.labelCharacters * 2} placeholder={gui.key_row_name_auto} onChange={event => { setLabel(event.target.value); setError(null); }} />
      </label>
      <SegmentedControl label={gui.key_row_type} value={kind} options={[
        { key: "chord", label: gui.key_row_chord }, { key: "sequence", label: gui.key_row_sequence },
      ]} onChange={next => { setKind(next); setActive(0); setError(null); }} />
      <div className="flex flex-col gap-in" data-key-entry-steps>
        {(kind === "chord" ? steps.slice(0, 1) : steps).map((step, index) => (
          <div key={index} data-key-entry-step={index} className="flex min-w-0 flex-col gap-in rounded-sm bg-fill p-in">
            {kind === "sequence" && (
              <div className="flex items-center gap-in">
                <span className="min-w-0 flex-1 text-callout tabular-nums">{index + 1} · {strokeCaption(step)}</span>
                {index > 0 && <Button size="icon" className="min-h-touch min-w-touch" data-key-step-up aria-label={gui.key_row_move_up} onClick={() => move(index, -1)}><IconChevronUp className="size-affordance" /></Button>}
                {index < steps.length - 1 && <Button size="icon" className="min-h-touch min-w-touch" data-key-step-down aria-label={gui.key_row_move_down} onClick={() => move(index, 1)}><IconChevronDown className="size-affordance" /></Button>}
                {steps.length > 1 && <Button size="icon" className="min-h-touch min-w-touch" data-key-step-remove aria-label={gui.key_row_remove_step} onClick={() => {update(steps.filter((_, at) => at !== index));setActive(0);}}><IconClose className="size-affordance" /></Button>}
              </div>
            )}
            <button type="button" data-key-step-select={index} aria-pressed={active===index} className={active===index?"min-h-touch rounded-sm bg-fill-strong px-in text-start text-body":"min-h-touch rounded-sm px-in text-start text-body"} onClick={()=>setActive(index)}>{strokeCaption(step)}</button>
          </div>
        ))}
        {kind === "sequence" && steps.length < KEY_ROW_LIMITS.steps && <Button className="min-h-touch self-start" data-key-step-add onClick={() => {setActive(steps.length);update([...steps, { key: "KeyN", ctrl: false, alt: false, shift: false }]);}}>{gui.key_row_add_step}</Button>}
      </div>
      <TerminalKeyboard stroke={steps[kind==="chord"?0:active]??steps[0]} onChange={stroke=>update(steps.map((step,index)=>index===(kind==="chord"?0:active)?stroke:step))}/>
      <div className="flex flex-col gap-in">
        <span data-key-entry-preview className="break-words text-body font-label">{label.trim() || caption}</span>
        {label.trim() && <span className="break-words text-callout text-muted-foreground">{caption}</span>}
        {error && <p role="alert" className="text-callout text-destructive">{error}</p>}
      </div>
      <div className="flex flex-wrap items-center gap-in">
        <Button data-key-entry-save className="min-h-touch" onClick={save}>{gui.key_row_save}</Button>
        <Button data-key-entry-cancel className="min-h-touch" variant="ghost" onClick={onCancel}>{gui.key_row_cancel}</Button>
        {onDelete && <Button data-key-entry-delete className="min-h-touch ml-auto" variant="ghost" onClick={onDelete}>{gui.delete}</Button>}
      </div>
    </section>
  );
}
