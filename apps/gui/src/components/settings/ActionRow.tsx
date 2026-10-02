// ActionRow — a setting that performs an action: label on the left,
// one button on the right. An act that asks again (a confirm, a form)
// says so with an ellipsis on its button (components/Button); one done
// on the spot does not.
//
// **A value may stand before the button**, the way a popup row shows
// its current value before its chevrons: 「store 在哪儿 — the path —
// 打开这个目录」 is one row, the fact and the act on it together.
// ⚠️ 墓碑 (2026-09-28): that was two rows with the same title, one
// holding the path and one the button.
//
// `data-setting-row="action"` is the gate anchor.
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export function ActionRow({
  name,
  label,
  hint,
  value,
  actionLabel,
  onPress,
  busy = false,
  danger = false,
  mark,
  rowMark,
}: {
  name: string;
  /** The row's title — words, or words with a mark of their own. */
  label: ReactNode;
  hint?: ReactNode;
  /** What the thing is now, in `secondary` before the button — the
      row's value, never a second title. */
  value?: ReactNode;
  actionLabel: string;
  onPress: () => void;
  /** The act is under way: the button breathes and takes no second
      press. ⚠️ 墓碑 (the audit fix plan F1.9): `disabled`, which drew the
      whole row at 0.4 — a row that cannot act now is not drawn at all,
      by its caller. */
  busy?: boolean;
  /** The act throws something away (网络's 离开…): the pill's word in
      the `destructive` ink. **Not the destructive fill** — that one is
      only the confirm window's (components/Button), and this row is the
      way to that window, not the act. */
  danger?: boolean;
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
      className="flex items-center gap-row px-row py-in"
    >
      {/* With a value the title keeps its width and the value takes
          what is left (a path wraps rather than crushing the title). */}
      <div
        className={cn(
          "flex flex-col gap-0",
          value === undefined ? "min-w-0 flex-1" : "flex-none",
        )}
      >
        <span className="text-body">{label}</span>
        {hint && (
          <span data-setting-note className="text-callout text-muted-foreground">{hint}</span>
        )}
      </div>
      {value !== undefined && (
        <span
          data-setting-value
          className={cn(
            "min-w-0 flex-1 [overflow-wrap:anywhere] text-right text-body text-muted-foreground",
          )}
        >
          {value}
        </span>
      )}
      <Button
        disabled={busy}
        aria-busy={busy || undefined}
        onClick={onPress}
        data-setting-trail
        {...mark}
        // Both axes of the finger floor: a two-character act (显示,
        // 修复) at `sm` is under it on the axis nobody thinks to check.
        className={cn("min-h-touch min-w-touch flex-none", danger && "text-destructive")}
      >
        {actionLabel}
      </Button>
    </div>
  );
}
