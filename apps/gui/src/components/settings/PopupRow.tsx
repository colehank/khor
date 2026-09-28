// PopupRow — a setting with a fixed set of choices, shown as `value ⌃⌄`
// and answered by the **system's** menu (docs/SHELL.md 第十七轮 §2,
// docs/design-system components/PopupButton): a native `<select>` sits
// transparent over the drawn button, so the OS draws the picker — never
// Radix, never a menu of our own.
//
// `data-setting-row="popup"` is the gate anchor; the `<select>` inside it
// is what a gate checks to prove the system menu is the one that opens.
import { IconChevronDown, IconChevronUp } from "@/components/icons";
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
          padding. */}
      <div className="group relative -mr-xxs flex-none">
        <div
          aria-hidden
          className={cn(
            // components/PopupButton: clear ground, `fill` under the
            // pointer, the value in `body` ink, and a `radius-xs` chip
            // holding the family's two chevrons at 10 — not the ⌃⌄ keys.
            // Disabled at the push button's 0.4, the label beside it too.
            "flex h-ctl-md items-center gap-in rounded-sm pr-xxs pl-in text-body",
            disabled ? "opacity-40" : "group-hover:bg-fill",
          )}
        >
          <span className="tabular-nums">{current}</span>
          <span data-setting-trail className="flex h-4.5 w-4 flex-none flex-col items-center justify-center rounded-xs bg-fill-strong text-muted-foreground">
            <IconChevronUp className="-mb-0.5 size-2.5" />
            <IconChevronDown className="-mt-0.5 size-2.5" />
          </span>
        </div>
        <select
          data-setting-select={name}
          value={value}
          disabled={disabled}
          aria-label={label}
          onChange={(e) => onPick(e.target.value)}
          // Centred on the drawn button and at least the finger floor
          // tall: the look is the pointer's, the hit area the finger's.
          className="absolute inset-x-0 top-1/2 h-full min-h-touch -translate-y-1/2 cursor-pointer opacity-0 disabled:cursor-default"
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
