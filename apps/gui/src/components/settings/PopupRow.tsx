// PopupRow — a fixed choice, rendered by the shared Khor choice menu.
// Rich previews, typography, focus and viewport rules live in PopupButton.
import type { ReactNode } from "react";

import { PopupButton, type PopupChoice } from "@/components/ui/popup-button";

export function PopupRow({
  name,
  label,
  hint,
  value,
  choices,
  onPick,
  mark,
  preview,
}: {
  name: string;
  label: string;
  hint?: string;
  value: string;
  choices: PopupChoice[];
  onPick: (value: string) => void;
  /** Extra anchors on the row (a gate reads the row's value off it). */
  mark?: Record<string, string>;
  /** What the choice looks like, drawn before the value — a setting
      that changes a drawing shows the drawing, not a sentence about it
      (the audit fix plan F6.4, SYS-19). */
  preview?: ReactNode;
}) {
  return (
    <div
      data-setting-row="popup"
      data-setting-row-name={name}
      {...mark}
      className="flex items-center gap-row px-row py-in"
    >
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="text-body">{label}</span>
        {hint && <span data-setting-note className="text-callout text-muted-foreground">{hint}</span>}
      </div>
      {/* `-mr-xxs`: the chevron chip — what is drawn at rest — ends on
          the row's trailing edge with the switches, buttons and values
          (`check-face` 4I); the `pr-xxs` inset is only seen under the
          pointer, where the `fill` may spill that far into the row's
          padding. The control itself is the one `PopupButton`. */}
      {preview && (
        <span data-setting-preview aria-hidden="true" className="flex flex-none">
          {preview}
        </span>
      )}
      <PopupButton
        value={value}
        choices={choices}
        onPick={onPick}
        label={label}
        mark={{ "data-setting-select": name }}
        className="-mr-xxs"
      />
    </div>
  );
}
