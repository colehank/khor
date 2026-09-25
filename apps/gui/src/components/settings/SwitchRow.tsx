// SwitchRow — a boolean setting: label on the left, a switch on the right
// (docs/SHELL.md 第十七轮 §2; docs/design-system components/Switch).
//
// The track is `--switch-w` × `--switch-h`; on is `--brand-strong` (one of
// the four places the brand voltage is allowed), off is `--fill-strong`.
// The knob is the track's height less a half-step on each side and wears
// the one exception shadow, `knob`. The button itself is the touch-sized
// hit target; the track is drawn inside it.
//
// `data-setting-row="switch"` is the gate anchor: every selectable
// setting on 更多 is one of the four row kinds.
import { cn } from "@/lib/utils";

export function SwitchRow({
  name,
  label,
  hint,
  on,
  onToggle,
  disabled = false,
  mark,
}: {
  /** Stable key — `data-setting-row-name`, never the word. */
  name: string;
  label: string;
  hint?: string;
  on: boolean;
  onToggle: () => void;
  disabled?: boolean;
  /** Extra gate anchors on the switch itself. */
  mark?: Record<string, string>;
}) {
  return (
    <div data-setting-row="switch" data-setting-row-name={name} className="flex min-h-row-1 items-center gap-row px-3 py-1.5">
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="text-body">{label}</span>
        {hint && <span className="text-callout text-muted-foreground">{hint}</span>}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={on}
        aria-label={label}
        data-on={on}
        disabled={disabled}
        onClick={onToggle}
        {...mark}
        className="-my-2 -mr-2 flex min-h-touch min-w-touch flex-none cursor-pointer items-center justify-center rounded-full focus-visible:outline-2 focus-visible:outline-offset-[-4px] focus-visible:outline-ring disabled:cursor-not-allowed disabled:opacity-50"
      >
        <span
          className={cn(
            "relative h-switch-h w-switch-w rounded-full transition-colors",
            on ? "bg-brand-strong" : "bg-fill-strong",
          )}
        >
          <span
            className={cn(
              "absolute top-0.5 bottom-0.5 aspect-square rounded-full bg-card shadow-knob transition-[left,right]",
              on ? "right-0.5" : "left-0.5",
            )}
          />
        </span>
      </button>
    </div>
  );
}
