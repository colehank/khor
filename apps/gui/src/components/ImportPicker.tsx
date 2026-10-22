/** Shared controls and result rows for importing sessions and subscriptions. */
import { useRef, type ButtonHTMLAttributes, type KeyboardEventHandler, type ReactNode, type RefObject } from "react";
import type { DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { machineChoice } from "@/components/MachineField";
import { ChoiceField } from "@/components/ui/choice-field";
import { IconClose, IconDevices, IconSearch } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { connectionKey, connectionKeyWord, connectionWord } from "@/words";

export type ImportMachine = { id: string; name: string; device?: DeviceRow };

export function importMachineAway(device: DeviceRow | undefined): string | null {
  if (!device) return null;
  const key = connectionKey(device);
  return key === "down" || key === "paused" ? connectionKeyWord(key) : null;
}

export function importMachineInScope(id: string, picked: ReadonlySet<string>, device?: DeviceRow): boolean {
  return picked.size === 0 ? importMachineAway(device) === null : picked.has(id);
}

export function toggleImportMachine(picked: ReadonlySet<string>, id: string | null): Set<string> {
  const next = new Set(picked);
  if (id === null) return new Set();
  if (!next.delete(id)) next.add(id);
  return next;
}

export function ImportMachinePicker({ machines, picked, onPick, disabled = false }: {
  machines: ImportMachine[];
  picked: ReadonlySet<string>;
  onPick: (id: string | null) => void;
  disabled?: boolean;
}) {
  const single = picked.size === 1 ? machines.find((m) => picked.has(m.id)) : undefined;
  return <ChoiceField
    label={gui.devices_tab}
    value={picked.size === 0 ? gui.sessions_all : single
      ? single.device?.shown ?? single.name : gui.import_machines_count(picked.size)}
    leadingIcon={single ? <MachineAvatar face={single.device?.face ?? null} className="size-kind-mark" /> : <IconDevices />}
    checked={picked.size === 0 ? new Set(["all"]) : picked}
    keepOpenOnPick
    options={[
      { id: "all", label: gui.sessions_all, icon: <IconDevices /> },
      ...machines.map((m) => machineChoice(
        m.device ?? { name: m.name, shown: m.name, face: null }, m.id,
        m.device ? connectionWord(m.device) || undefined : undefined,
      )),
    ]}
    onPick={(id) => onPick(id === "all" ? null : id)}
    disabled={disabled}
  />;
}

export function ImportSearch({ value, onChange, label, inputRef, onKeyDown }: {
  value: string;
  onChange: (value: string) => void;
  label: string;
  inputRef?: RefObject<HTMLInputElement | null>;
  onKeyDown?: KeyboardEventHandler<HTMLInputElement>;
}) {
  const ownField = useRef<HTMLInputElement>(null);
  const field = inputRef ?? ownField;
  return <div data-import-find className="relative flex-none">
    <IconSearch className="pointer-events-none absolute top-1/2 left-in size-affordance -translate-y-1/2 text-tertiary" />
    <input
      ref={field}
      data-import-field
      data-slot="input"
      type="search"
      enterKeyHint="go"
      autoCapitalize="none"
      autoCorrect="off"
      spellCheck={false}
      placeholder={label}
      aria-label={label}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={onKeyDown}
      className="h-ctl-md min-h-touch w-full min-w-0 appearance-none rounded-sm border-0 bg-fill ps-7 pe-8 text-body text-foreground placeholder:text-tertiary focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring [&::-webkit-search-cancel-button]:hidden"
    />
    {value !== "" && <button
      type="button"
      data-import-clear
      aria-label={gui.search_clear}
      onPointerDown={(e) => e.preventDefault()}
      onMouseDown={(e) => e.preventDefault()}
      onClick={() => { onChange(""); field.current?.focus(); }}
      className="absolute top-1/2 right-1 grid size-touch -translate-y-1/2 place-items-center"
    >
      <span className="grid size-5 place-items-center rounded-full bg-fill-strong text-muted-foreground"><IconClose className="size-3" /></span>
    </button>}
  </div>;
}

export function ImportResult({ glyph, title, detail, detailTitle, command, active, busy, className, ...props }: Omit<ButtonHTMLAttributes<HTMLButtonElement>, "title"> & {
  glyph: ReactNode;
  title: ReactNode;
  detail: ReactNode;
  detailTitle?: string;
  command?: ReactNode;
  active?: boolean;
  busy?: boolean;
}) {
  return <button
    type="button"
    data-import-row=""
    {...props}
    data-active={active || undefined}
    data-on={busy || undefined}
    aria-busy={busy || undefined}
    className={cn("flex min-h-row-2 w-full min-w-0 flex-none items-center gap-row rounded-md px-in py-xxs text-left hover:bg-fill data-[active]:bg-fill data-[on=true]:bg-fill-strong", className)}
  >
    <span data-import-glyph className="grid size-8 flex-none place-items-center text-muted-foreground">{glyph}</span>
    <span className="flex min-w-0 flex-1 flex-col">
      <span className="flex min-w-0 items-baseline gap-row">
        <span data-import-title className="min-w-0 flex-[0_1_auto] truncate text-body">{title}</span>
        {command && <span data-import-command className="min-w-[35%] flex-[1_1_0%] truncate text-right font-mono text-caption text-muted-foreground">{command}</span>}
      </span>
      <span data-import-where title={detailTitle} className="truncate text-callout text-muted-foreground">{detail}</span>
    </span>
  </button>;
}

export function ImportCalm({ sentence, hint, mark }: { sentence: string; hint?: string; mark: Record<string, string> }) {
  return <div {...mark} className="flex min-h-40 flex-1 flex-col items-center justify-center gap-xxs px-base text-center">
    <span className="text-body text-muted-foreground">{sentence}</span>
    {hint && <span className="text-callout text-muted-foreground">{hint}</span>}
  </div>;
}
