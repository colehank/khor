// PopupRow — a setting with a fixed set of choices, shown as `value ⌃⌄`
// and answered by the **system's** menu (docs/SHELL.md 第十七轮 §2,
// docs/design-system components/PopupButton): a native `<select>` sits
// transparent over the drawn button, so the OS draws the picker — never
// Radix, never a menu of our own.
//
// `data-setting-row="popup"` is the gate anchor; the `<select>` inside it
// is what a gate checks to prove the system menu is the one that opens.
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
  const current = choices.find((c) => c.value === value)?.label ?? value;
  return (
    <div
      data-setting-row="popup"
      data-setting-row-name={name}
      {...mark}
      className="flex min-h-row-1 items-center gap-row px-3 py-1.5"
    >
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="text-body">{label}</span>
        {hint && <span className="text-callout text-muted-foreground">{hint}</span>}
      </div>
      <div className="relative flex-none">
        <div
          aria-hidden
          className={cn(
            "flex h-ctl-sm items-center gap-1 rounded-sm bg-fill px-2 text-callout text-secondary-foreground",
            disabled && "opacity-50",
          )}
        >
          <span className="tabular-nums">{current}</span>
          <span className="text-muted-foreground">⌃⌄</span>
        </div>
        <select
          data-setting-select={name}
          value={value}
          disabled={disabled}
          aria-label={label}
          onChange={(e) => onPick(e.target.value)}
          // Centred on the drawn button and at least the finger floor
          // tall: the look is the pointer's, the hit area the finger's.
          className="absolute inset-x-0 top-1/2 h-full min-h-touch -translate-y-1/2 cursor-pointer opacity-0"
        >
          {choices.map((c) => (
            <option key={c.value} value={c.value}>
              {c.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}
