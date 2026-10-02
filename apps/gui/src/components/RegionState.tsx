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
 *
 * **An empty landing creates nothing here** (2026-10-01, the owner:
 * 「空状态不加添加工作区和登记」): an act that makes a thing has one home,
 * the ＋ beside the search box (10-layout D9). Where the sentence would
 * leave a person asking 「how do I get one」, a `hint` under it says
 * where that ＋ is — words, not a second door. `action` stays for 重试,
 * which redoes what failed rather than making anything.
 */
export function EmptyState({
  headline,
  action,
  hint,
  state = "empty",
  className,
  children,
  ...marks
}: {
  headline?: ReactNode;
  /** At most one button — 重试, never an act that makes something. */
  action?: ReactNode;
  /** A second line under the sentence, `callout`: where the act that
      would fill this lives (「点击搜索框旁的 + …」). */
  hint?: ReactNode;
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
      {/* The sentence and its hint are one statement: `gap-xxs` between
          them, where the button below stood `gap-row` off. */}
      <div className="flex flex-col items-center gap-xxs">
        {/* A `div`, not a `p`: callers hand in blocks (the workspace
            list's sentence and its hint), and a block inside a paragraph
            is invalid markup React reports on every render. */}
        <div className="max-w-measure text-body text-muted-foreground">{children}</div>
        {hint && <p data-empty-hint className="max-w-measure text-callout text-muted-foreground">{hint}</p>}
      </div>
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
