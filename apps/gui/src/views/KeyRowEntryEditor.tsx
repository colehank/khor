import { useState } from "react";
import { Terminal } from "@xterm/xterm";

import { IconChevronDown, IconChevronUp, IconClose } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PopupButton } from "@/components/ui/popup-button";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { gui } from "@/gen/catalog";
import { actionCaption, baseKeyCaption, encodeKeyAction, strokeCaption } from "@/lib/term-key-codec";
import { BASE_KEYS, KEY_ROW_LIMITS, type BaseKey, type CustomKeyEntry, type KeyStroke } from "@/lib/term-key-model";

const emptyStroke = (): KeyStroke => ({ key: "ArrowLeft", ctrl: false, alt: false, shift: true });
const bytesCaption = (bytes: string) => JSON.stringify(bytes).slice(1, -1);

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
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const action = kind === "chord" ? { kind, stroke: steps[0] } as const : { kind, steps } as const;
  const caption = actionCaption(action);
  const update = (next: KeyStroke[]) => { setSteps(next); setResult(null); setError(null); };
  const move = (at: number, by: number) => {
    const next = [...steps];
    [next[at], next[at + by]] = [next[at + by], next[at]];
    update(next);
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
  const test = () => {
    const isolated = new Terminal();
    try { setResult(bytesCaption(encodeKeyAction(isolated, action))); setError(null); }
    catch { setResult(null); setError(gui.key_row_unsupported); }
    finally { isolated.dispose(); }
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
      ]} onChange={next => { setKind(next); setResult(null); setError(null); }} />
      <div className="flex flex-col gap-in" data-key-entry-steps>
        {(kind === "chord" ? steps.slice(0, 1) : steps).map((step, index) => (
          <div key={index} data-key-entry-step={index} className="flex min-w-0 flex-col gap-in rounded-sm bg-fill p-in">
            {kind === "sequence" && (
              <div className="flex items-center gap-in">
                <span className="min-w-0 flex-1 text-callout tabular-nums">{index + 1} · {strokeCaption(step)}</span>
                {index > 0 && <Button size="icon" className="min-h-touch min-w-touch" data-key-step-up aria-label={gui.key_row_move_up} onClick={() => move(index, -1)}><IconChevronUp className="size-affordance" /></Button>}
                {index < steps.length - 1 && <Button size="icon" className="min-h-touch min-w-touch" data-key-step-down aria-label={gui.key_row_move_down} onClick={() => move(index, 1)}><IconChevronDown className="size-affordance" /></Button>}
                {steps.length > 1 && <Button size="icon" className="min-h-touch min-w-touch" data-key-step-remove aria-label={gui.key_row_remove_step} onClick={() => update(steps.filter((_, at) => at !== index))}><IconClose className="size-affordance" /></Button>}
              </div>
            )}
            <PopupButton label={gui.key_row_base_key} look="field" value={step.key} choices={BASE_KEYS.map(key => ({ value: key, label: baseKeyCaption(key) }))} mark={{ "data-key-entry-base": String(index) }} onPick={key => update(steps.map((value, at) => at === index ? { ...value, key: key as BaseKey } : value))} />
            <div className="flex flex-wrap gap-in" role="group" aria-label={gui.key_row_chord}>
              {(["ctrl", "alt", "shift"] as const).map(modifier => (
                <Button key={modifier} className="min-h-touch" data-key-modifier={modifier} aria-pressed={step[modifier]} onClick={() => update(steps.map((value, at) => at === index ? { ...value, [modifier]: !value[modifier] } : value))} variant={step[modifier] ? "secondary" : "default"}>
                  {{ ctrl: "Ctrl", alt: "Alt", shift: "Shift" }[modifier]}
                </Button>
              ))}
            </div>
          </div>
        ))}
        {kind === "sequence" && steps.length < KEY_ROW_LIMITS.steps && <Button className="min-h-touch self-start" data-key-step-add onClick={() => update([...steps, { key: "KeyN", ctrl: false, alt: false, shift: false }])}>{gui.key_row_add_step}</Button>}
      </div>
      <div className="flex flex-col gap-in">
        <span data-key-entry-preview className="break-words text-body font-label">{label.trim() || caption}</span>
        {label.trim() && <span className="break-words text-callout text-muted-foreground">{caption}</span>}
        <p className="text-callout text-muted-foreground">{gui.key_row_test_note}</p>
        <Button className="min-h-touch self-start" data-key-entry-test onClick={test}>{gui.key_row_test}</Button>
        {result !== null && <output data-key-entry-output aria-label={gui.key_row_test_output} className="break-all font-mono text-callout">{result}</output>}
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
