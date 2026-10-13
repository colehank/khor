// **The six words, one look** (docs/design-system components/StatusWord):
// a dot and a word in `caption`, both in the word's own ink. The ink is
// on the one `data-word-text` span, so selection contrast and busy's
// breath reach the dot and the word together, without losing the hue.
//
// The word is the catalog's (`words`), never a phrasing of the caller's.
import { cn } from "@/lib/utils";
import { word } from "@/words";
import type { CSSProperties } from "react";

export function StatusWord({
  word: w,
  dot = true,
  className,
  ...marks
}: {
  word: string;
  /** The dot is left off where the word stands in a line of its own kind. */
  dot?: boolean;
  className?: string;
  /** The caller's own anchors (`data-*`). */
  [mark: `data-${string}`]: unknown;
}) {
  if (!w) return <span {...(marks as Record<string, unknown>)} data-status-unavailable className={cn("text-caption text-secondary-foreground", className)}>{word(w)}</span>;
  return (
    <span
      {...(marks as Record<string, unknown>)}
      data-word-text
      data-status-word={w}
      style={{
        "--status-ink": `var(--state-${w})`,
        "--status-selected-ink": `var(--state-selected-${w}, var(--state-${w}))`,
      } as CSSProperties}
      className={cn("inline-flex flex-none items-center gap-xxs text-caption", className)}
    >
      {dot && <span aria-hidden className="size-1.5 flex-none rounded-full bg-current" />}
      {word(w)}
    </span>
  );
}
