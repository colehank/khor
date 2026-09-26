// ActionRow — a setting that performs an action: label on the left,
// one button on the right.
//
// `data-setting-row="action"` is the gate anchor.
import { Button } from "@/components/ui/button";

export function ActionRow({
  name,
  label,
  hint,
  actionLabel,
  onPress,
  disabled = false,
  variant = "default",
  mark,
}: {
  name: string;
  label: string;
  hint?: string;
  actionLabel: string;
  onPress: () => void;
  disabled?: boolean;
  variant?: "default" | "ghost" | "destructive";
  /** Extra anchors on the button. */
  mark?: Record<string, string>;
}) {
  return (
    <div
      data-setting-row="action"
      data-setting-row-name={name}
      className="flex min-h-row-1 items-center gap-row px-row py-in"
    >
      <div className="flex min-w-0 flex-1 flex-col gap-0">
        <span className="text-body">{label}</span>
        {hint && (
          <span className="text-callout text-muted-foreground">{hint}</span>
        )}
      </div>
      <Button
        variant={variant}
        size="sm"
        disabled={disabled}
        onClick={onPress}
        {...mark}
        className="min-h-touch flex-none"
      >
        {actionLabel}
      </Button>
    </div>
  );
}
