// Switch — the one on/off look in the app (docs/design-system
// components/Switch): a `--switch-w` × `--switch-h` track, `--brand-strong`
// when on (one of the places the brand voltage is allowed) and
// `--fill-strong` when off, and a white knob wearing the one exception
// shadow, `knob`. The button itself is the touch-sized hit target; the
// track is drawn inside it.
//
// The switch carries no words: its title sits beside it (a settings row,
// a chat option's name), and `label` is what a screen reader hears.
import { cn } from "@/lib/utils";

export function Switch({
  on,
  label,
  onToggle,
  disabled = false,
  mark,
  className,
}: {
  on: boolean;
  /** What a screen reader hears — the title beside the switch. */
  label: string;
  onToggle: () => void;
  disabled?: boolean;
  /** Extra anchors on the switch itself. */
  mark?: Record<string, string | boolean | undefined>;
  className?: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      data-on={on}
      disabled={disabled}
      onClick={onToggle}
      {...mark}
      className={cn(
        "flex min-h-touch min-w-touch flex-none cursor-pointer items-center justify-center rounded-full focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
    >
      <span
        // The drawn control, as opposed to the hit area around it — what
        // a settings row lines up on its trailing edge (`check-face` 4I).
        data-switch-track
        className={cn(
          "relative h-switch-h w-switch-w rounded-full transition-colors",
          on ? "bg-brand-strong" : "bg-fill-strong",
        )}
      >
        <span
          className={cn(
            "absolute top-0.5 bottom-0.5 aspect-square rounded-full bg-white shadow-knob transition-[left,right]",
            on ? "right-0.5" : "left-0.5",
          )}
        />
      </span>
    </button>
  );
}
