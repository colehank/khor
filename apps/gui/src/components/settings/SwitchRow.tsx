// SwitchRow — a boolean setting: label on the left, a switch on the right
// (docs/SHELL.md 第十七轮 §2; docs/design-system components/Switch).
//
// The switch is `Switch`, the one on/off look — the chat's option
// switches wear the same one.
//
// `data-setting-row="switch"` is the gate anchor: every selectable
// setting on 更多 is one of the four row kinds.
import type { ReactNode } from "react";

import { Switch } from "@/components/settings/Switch";

export function SwitchRow({
  name,
  label,
  hint,
  on,
  onToggle,
  disabled = false,
  mark,
  rowMark,
}: {
  /** Stable key — `data-setting-row-name`, never the word. */
  name: string;
  label: string;
  hint?: ReactNode;
  on: boolean;
  onToggle: () => void;
  disabled?: boolean;
  /** Extra gate anchors on the switch itself. */
  mark?: Record<string, string>;
  /** Extra gate anchors on the row (a config key's name and value). */
  rowMark?: Record<string, string>;
}) {
  return (
    <div
      data-setting-row="switch"
      data-setting-row-name={name}
      {...rowMark}
      className="flex min-h-row-1 items-center gap-row px-row py-in"
    >
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="text-body">{label}</span>
        {hint && <span className="text-callout text-muted-foreground">{hint}</span>}
      </div>
      <Switch on={on} label={label} onToggle={onToggle} disabled={disabled} mark={mark} className="-my-2 -mr-2" />
    </div>
  );
}
