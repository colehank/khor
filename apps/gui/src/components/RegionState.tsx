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
import { useEffect, useState, type ReactNode } from "react";

import { IconReload } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { lasting } from "@/words";

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

/** How long a wait goes before a spinner joins the skeleton
    (guidelines/20-states: 「不到 1 秒不出转圈」). */
export const WAIT_SPIN_MS = 1000;

/** How long a wait goes before it is said in words, with a way out. A
    machine on a working wire answers in well under a second; one that
    is not answering is known to be dead within the node's ack wait (4 s,
    `crate::wires::ACK_WITHIN`) or its dial (10 s) — at eight a person
    has waited long enough to be told, and to be offered the two moves. */
export const WAIT_SAY_MS = 8000;

/** Milliseconds since `since`, ticking once a second while it is set;
    0 when it is not. */
export function useWaited(since: number | null): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (since === null) return;
    setNow(Date.now());
    const t = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(t);
  }, [since]);
  return since === null ? 0 : Math.max(0, now - since);
}

/**
 * **A region waiting on another machine** (round NET-FIX, audit/net.md
 * F9): the never-seen skeleton at once, so 300 ms never passes over a
 * blank board; a spinner under it from [`WAIT_SPIN_MS`]; and from
 * [`WAIT_SAY_MS`] the sentence — which machine, how long — with
 * 停止等待 and 重试. Nothing before it is needed: the two buttons are drawn
 * only once there is something to decide (渐进式披露).
 *
 * `onCancel` stops waiting (the request runs out on its own deadline in
 * the node); `onRetry` asks again from the start. Either may be left out
 * where it would mean nothing.
 */
export function Waiting({
  since,
  machine,
  onCancel,
  onRetry,
  rows = true,
}: {
  /** When the wait began (`Date.now()`). */
  since: number;
  /** The machine being waited on, by the name a person reads. */
  machine?: string | null;
  onCancel?: () => void;
  onRetry?: () => void;
  /** Whether to draw the skeleton rows — a pane that keeps its last
      content passes `false` and gets the spinner and sentence alone. */
  rows?: boolean;
}) {
  const waited = useWaited(since);
  const spin = waited >= WAIT_SPIN_MS;
  const say = waited >= WAIT_SAY_MS;
  return (
    <div data-region-state="waiting" aria-busy="true" className="flex flex-col gap-base p-pane">
      {rows && (
        <div className="flex flex-col gap-in">
          {SKELETON.map((w, i) => (
            <div key={i} data-skeleton className={cn("h-row-2 rounded-md bg-fill", w)} />
          ))}
        </div>
      )}
      {spin && (
        <div data-waiting={say ? "said" : "spinning"} className="flex flex-col items-center gap-row text-center">
          <IconReload className="size-5 flex-none animate-spin text-muted-foreground motion-reduce:animate-none" />
          {say && (
            <>
              <p role="status" className="max-w-measure text-callout text-muted-foreground tabular-nums">
                {machine ? gui.waiting_on(machine, lasting(waited)) : gui.waiting(lasting(waited))}
              </p>
              {(onCancel || onRetry) && (
                <div className="flex gap-in">
                  {onCancel && (
                    <Button data-waiting-cancel onClick={onCancel}>
                      {gui.waiting_cancel}
                    </Button>
                  )}
                  {onRetry && (
                    <Button data-waiting-retry onClick={onRetry}>
                      {gui.region_retry}
                    </Button>
                  )}
                </div>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
