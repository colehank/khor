// PopupRow — a setting with a fixed set of choices, shown as `value ⌃⌄`
// and answered by the **system's** menu (docs/SHELL.md 第十七轮 §2,
// docs/design-system components/PopupButton): a native `<select>` sits
// transparent over the drawn button, so the OS draws the picker — never
// Radix, never a menu of our own.
//
// `data-setting-row="popup"` is the gate anchor; the `<select>` inside it
// is what a gate checks to prove the system menu is the one that opens.
import { PopupButton } from "@/components/ui/popup-button";
import { cn } from "@/lib/utils";

export function PopupRow({
  name,
  label,
  hint,
  value,
  choices,
  onPick,
  disabled = false,
  mark,
}: {
  name: string;
  label: string;
  hint?: string;
  value: string;
  choices: { value: string; label: string }[];
  onPick: (value: string) => void;
  disabled?: boolean;
  /** Extra anchors on the row (a gate reads the row's value off it). */
  mark?: Record<string, string>;
}) {
  return (
    <div
      data-setting-row="popup"
      data-setting-row-name={name}
      {...mark}
      className="flex items-center gap-row px-row py-in"
    >
      <div className={cn("flex min-w-0 flex-1 flex-col", disabled && "opacity-40")}>
        <span className="text-body">{label}</span>
        {hint && <span data-setting-note className="text-callout text-muted-foreground">{hint}</span>}
      </div>
      {/* `-mr-xxs`: the chevron chip — what is drawn at rest — ends on
          the row's trailing edge with the switches, buttons and values
          (`check-face` 4I); the `pr-xxs` inset is only seen under the
          pointer, where the `fill` may spill that far into the row's
          padding. The control itself is the one `PopupButton`. */}
      <PopupButton
        value={value}
        choices={choices}
        onPick={onPick}
        label={label}
        disabled={disabled}
        mark={{ "data-setting-select": name }}
        className="-mr-xxs"
      />
    </div>
  );
}
