// **Five states of a region, one look each, app-wide**
// (docs/SHELL.md 第十七轮 §6; docs/design-system guidelines/20-states.md,
// components/RegionState).
//
// A region — the board, a pane in it, a settings group — is in exactly
// one of these at any moment, and its look comes from here rather than
// from each page:
//
//   loading-never  never seen: skeleton rows on `fill`, no spinner
//   loading-seen   refreshing: last content kept; R0 carries a 2 px
//                  brand bar (`Region busy`), nothing is drawn here
//   stale          stale / offline: last content kept; R0's right end
//                  carries a caption (`Region caption`), never a banner
//   empty          one sentence, centred, at most one button
//   error          the machine's own sentence where the empty state
//                  sits, and 重试
//
// The two that keep content draw the content untouched: the bar and the
// caption live in R0, which is the `Region`'s, so a page passes `busy` /
// `caption` there and hands its content through here unchanged.
import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

const SKELETON = ["w-[72%]", "w-[56%]", "w-[64%]", "w-[40%]"];

/**
 * **The one place for 空、出错、还没选中** (components/EmptyState): centred
 * in its region, one sentence in `body` / `secondary`, at most one
 * button. A `headline` in `title-lg` is rare on purpose — it is the
 * largest size in the app. No illustration, no red box, no stack.
 */
export function EmptyState({
  headline,
  action,
  state = "empty",
  className,
  children,
  ...marks
}: {
  headline?: ReactNode;
  /** At most one button. */
  action?: ReactNode;
  state?: "empty" | "error";
  className?: string;
  children?: ReactNode;
  /** The caller's own anchors (`data-*`), laid on the block. */
  [mark: `data-${string}`]: unknown;
}) {
  return (
    <div
      {...(marks as Record<string, unknown>)}
      data-region-state={state}
      data-empty-state
      className={cn("flex h-full min-h-0 flex-1 flex-col items-center justify-center gap-row p-pane text-center", className)}
    >
      {headline && <p className="text-title-lg font-title text-foreground">{headline}</p>}
      <p className="max-w-measure text-body text-muted-foreground">{children}</p>
      {action}
    </div>
  );
}

export type RegionStateKind = "loading-never" | "loading-seen" | "stale" | "empty" | "error" | "ready";

export function RegionState({
  state,
  message,
  action,
  onRetry,
  children,
}: {
  state: RegionStateKind;
  /** The sentence of an empty or error state. */
  message?: ReactNode;
  /** An empty state's one button, if it has one. */
  action?: ReactNode;
  /** An error state's 重试. */
  onRetry?: () => void;
  /** The content, for the states that show it. */
  children?: ReactNode;
}) {
  if (state === "loading-never") {
    // The skeleton has the shape of what is coming (components/RegionState:
    // 「骨架的形状要和真内容的形状一样」): four two-line rows at uneven
    // widths, `gap-in` apart, on `fill`.
    return (
      <div data-region-state={state} aria-busy="true" className="flex flex-col gap-in p-pane">
        {SKELETON.map((w, i) => (
          <div key={i} data-skeleton className={cn("h-row-2 rounded-md bg-fill", w)} />
        ))}
      </div>
    );
  }
  if (state === "empty" || state === "error") {
    return (
      <EmptyState
        state={state}
        action={
          state === "error" && onRetry ? (
            <Button data-region-retry onClick={onRetry}>
              {gui.region_retry}
            </Button>
          ) : (
            action
          )
        }
      >
        {message}
      </EmptyState>
    );
  }
  return (
    <div data-region-state={state} className="contents">
      {children}
    </div>
  );
}
