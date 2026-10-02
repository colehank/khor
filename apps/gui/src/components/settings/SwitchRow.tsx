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
  mark,
  rowMark,
}: {
  /** Stable key — `data-setting-row-name`, never the word. */
  name: string;
  label: string;
  hint?: ReactNode;
  on: boolean;
  onToggle: () => void;
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
      className="flex items-center gap-row px-row py-in"
    >
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="text-body">{label}</span>
        {hint && <span data-setting-note className="text-callout text-muted-foreground">{hint}</span>}
      </div>
      {/* **The track ends on the row's trailing edge**, where every
          other row kind ends its control (`check-face` 4I). The hit box
          grows to the left of it, never past it: ⚠️ 墓碑 (2026-09-28) —
          a `-mr-2` here pulled the box right so a finger's 44 would
          centre on the track, and with a mouse the box *is* the track,
          so every switch stood 8 past the popups and buttons above it. */}
      <Switch on={on} label={label} onToggle={onToggle} mark={mark} className="-my-2 justify-end" />
    </div>
  );
}
