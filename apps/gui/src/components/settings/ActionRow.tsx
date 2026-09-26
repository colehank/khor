// ActionRow — a setting that performs an action: label on the left,
// one button on the right. An act that asks again (a confirm, a form)
// says so with an ellipsis on its button (components/Button); one done
// on the spot does not.
//
// `data-setting-row="action"` is the gate anchor.
import type { ReactNode } from "react";

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
  rowMark,
}: {
  name: string;
  /** The row's title — words, or words with a mark of their own. */
  label: ReactNode;
  hint?: ReactNode;
  actionLabel: string;
  onPress: () => void;
  disabled?: boolean;
  variant?: "default" | "ghost" | "destructive";
  /** Extra anchors on the button. */
  mark?: Record<string, string>;
  /** Extra anchors on the row itself. */
  rowMark?: Record<string, string>;
}) {
  return (
    <div
      data-setting-row="action"
      data-setting-row-name={name}
      {...rowMark}
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
        // Both axes of the finger floor: a two-character act (显示,
        // 修复) at `sm` is under it on the axis nobody thinks to check.
        className="min-h-touch min-w-touch flex-none"
      >
        {actionLabel}
      </Button>
    </div>
  );
}
