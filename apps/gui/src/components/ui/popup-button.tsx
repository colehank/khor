// PopupButton — 「从几个里选一个」 (docs/design-system components/PopupButton):
// the current value, and a `radius-xs` chip holding the family's two
// chevrons at its trailing end. **The menu is the system's**: a native
// `<select>` sits transparent over the drawn button, so the OS draws the
// picker, its keyboard, its scrolling and its long-list behaviour — never
// Radix, never a menu of our own.
//
// Two looks, one control:
//
//   - `plain` — a settings row's trailing control (`PopupRow`): clear
//     ground, `fill` under the pointer, as wide as its value.
//   - `field` — a row of a form in a sheet (新会话's 目标机器, 导入's two
//     filters): the `fill` ground a search field and a segmented
//     control's track wear, `radius-sm`, as wide as the column, so the
//     sheet's controls read as one family and the ⌃⌄ stands at the
//     field's end where a person looks for it.
//
// `data-popup-button` is on the drawn box; the `<select>` inside carries
// whatever `mark` the caller hands it, and each `<option>` its own marks
// — a gate picks by `selectOption`, and reads `data-on` off the options.
import { IconChevronDown, IconChevronUp } from "@/components/icons";
import { cn } from "@/lib/utils";

export type PopupChoice = {
  value: string;
  /** The words in the system's menu. */
  label: string;
  /** What the button shows when this is the chosen one, where the
      menu's words are too long for it (a path's last part). */
  short?: string;
  disabled?: boolean;
  /** Anchors on this choice's `<option>`. */
  mark?: Record<string, string | boolean | undefined>;
};

export function PopupButton({
  value,
  choices,
  onPick,
  label,
  disabled = false,
  look = "plain",
  mark,
  className,
}: {
  value: string;
  choices: PopupChoice[];
  onPick: (value: string) => void;
  /** What a screen reader hears — the row's or the field's label. */
  label: string;
  disabled?: boolean;
  look?: "plain" | "field";
  /** Anchors on the `<select>` itself. */
  mark?: Record<string, string | boolean | undefined>;
  className?: string;
}) {
  const chosen = choices.find((c) => c.value === value);
  const current = chosen ? (chosen.short ?? chosen.label) : value;
  return (
    <div
      data-popup-button={look}
      className={cn("group relative min-w-0", look === "field" ? "flex w-full" : "flex-none", className)}
    >
      <div
        aria-hidden
        className={cn(
          // components/PopupButton: the value in `body` ink (never
          // `secondary` — that is a settings row's note), and a
          // `radius-xs` chip holding the two chevrons at 10 — not the
          // ⌃⌄ keys. Disabled at the push button's 0.4.
          "flex h-ctl-md min-w-0 items-center gap-in rounded-sm pr-xxs pl-in text-body",
          look === "field" ? "w-full bg-fill" : "",
          disabled ? "opacity-40" : look === "field" ? "group-hover:bg-fill-strong" : "group-hover:bg-fill",
        )}
      >
        <span className={cn("min-w-0 truncate tabular-nums", look === "field" && "flex-1")}>{current}</span>
        <span
          data-setting-trail
          className="flex h-4.5 w-4 flex-none flex-col items-center justify-center rounded-xs bg-fill-strong text-muted-foreground"
        >
          <IconChevronUp className="-mb-0.5 size-2.5" />
          <IconChevronDown className="-mt-0.5 size-2.5" />
        </span>
      </div>
      <select
        value={value}
        disabled={disabled}
        aria-label={label}
        onChange={(e) => onPick(e.target.value)}
        {...mark}
        // Centred on the drawn button and at least the finger floor
        // tall: the look is the pointer's, the hit area the finger's.
        className="absolute inset-x-0 top-1/2 h-full min-h-touch w-full -translate-y-1/2 cursor-pointer opacity-0 disabled:cursor-default"
      >
        {choices.map((c) => (
          <option key={c.value} value={c.value} disabled={c.disabled} data-on={c.value === value} {...c.mark}>
            {c.label}
          </option>
        ))}
      </select>
    </div>
  );
}
