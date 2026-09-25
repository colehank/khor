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
    return (
      <div data-region-state={state} aria-busy="true" className="flex flex-col gap-row p-pane">
        {[0, 1, 2].map((i) => (
          <div key={i} data-skeleton className="h-row-1 rounded-md bg-fill" />
        ))}
      </div>
    );
  }
  if (state === "empty" || state === "error") {
    return (
      <div
        data-region-state={state}
        className="flex h-full min-h-0 flex-col items-center justify-center gap-base p-pane text-center"
      >
        <p className="max-w-measure text-title-lg text-muted-foreground">{message}</p>
        {state === "error" && onRetry ? (
          <Button data-region-retry onClick={onRetry} className="min-h-touch">
            {gui.region_retry}
          </Button>
        ) : (
          action
        )}
      </div>
    );
  }
  return (
    <div data-region-state={state} className="contents">
      {children}
    </div>
  );
}
