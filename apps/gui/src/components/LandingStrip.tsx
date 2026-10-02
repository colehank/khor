/**
 * **The wide face's navigation: the landings' glyphs in the window's top
 * row** (the redesign, 2026-10-02, the owner's 「方案 B」). One glass
 * capsule beside the window's buttons — **the largest control in that
 * row** (the owner: it read smaller than the session tabs beside it), 36
 * high with 36 × 30 pills and 22 glyphs — holding the landings in the
 * user's order (`lib/nav-order`, all five), then, after a thin rule, 更多.
 * The open one is a `fill-strong` pill with the brand's deep ink and its
 * solid piece filled (`rail-on-ink`, `app.css`); 更多 is lit the same way
 * while its pages are open. A glyph's name is the hint a pointer gets.
 *
 * **Pressing the open landing again folds its list away**, and once more
 * brings it back; another glyph switches and brings it back. The shell
 * does the folding. Folded, the strip stays where it was on screen — the
 * window's top-left — at the start of the content's first row
 * (`Region`'s lead).
 *
 * `MeButton` is the row's other end: this machine's face, going straight
 * to 这台机器. The narrow face has neither: its navigation floats
 * (`FloatNav`).
 */
import type { ReactNode } from "react";

import { MachineAvatar } from "@/components/Avatar";
import { IconMore } from "@/components/icons";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { Avatar } from "@/gen/bindings/Avatar";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

export type StripSeat = { key: string; name: string; glyph: ReactNode; badge?: number };

function Seat({
  label,
  on,
  mark,
  onClick,
  children,
}: {
  label: string;
  on: boolean;
  mark: Record<string, string>;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          data-slot="button"
          data-rail-item
          data-on={on}
          aria-label={label}
          aria-current={on ? "page" : undefined}
          onClick={onClick}
          {...mark}
          className={cn(
            "relative grid h-strip-item-h w-strip-item place-items-center rounded-full text-rail-ink hover:bg-fill",
            "focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring [&>svg]:size-strip-glyph",
            on && "glass-sel text-[var(--rail-on-ink)] hover:glass-sel",
          )}
        >
          {children}
        </button>
      </TooltipTrigger>
      <TooltipContent side="bottom" data-rail-tip>
        {label}
      </TooltipContent>
    </Tooltip>
  );
}

export function LandingStrip({
  seats,
  active,
  moreOn,
  onGo,
  onMore,
}: {
  seats: StripSeat[];
  /** The open landing, or null while a 「…」 page is up. */
  active: string | null;
  /** Whether 更多's pages are the thing on screen. */
  moreOn: boolean;
  onGo: (key: string) => void;
  onMore: () => void;
}) {
  return (
    <nav
      data-landing-strip
      aria-label={gui.nav}
      className="glass flex h-strip-h flex-none items-center rounded-full px-glass-pad"
    >
      {seats.map((s) => (
        <Seat key={s.key} label={s.name} on={active === s.key} mark={{ "data-landing": s.key }} onClick={() => onGo(s.key)}>
          {s.glyph}
          {s.badge !== undefined && s.badge > 0 && (
            <span className="absolute -top-1 -right-1 min-w-4 rounded-full bg-badge px-1 text-center text-caption leading-4 tabular-nums text-badge-foreground">
              {s.badge}
            </span>
          )}
        </Seat>
      ))}
      {/* 更多 is a place too, not a landing: a thin rule keeps it apart
          (the owner, 2026-10-02: 「把更多带回来」). */}
      <span aria-hidden="true" className="mx-0.5 h-4.5 w-px flex-none bg-border" />
      <Seat label={gui.rail_more} on={moreOn} mark={{ "data-rail-more": "" }} onClick={onMore}>
        <IconMore />
      </Seat>
    </nav>
  );
}

/** This machine's face at the far end of the top row, straight to
    这台机器 — in the list column's row, and in the content's while the
    list is folded. */
export function MeButton({
  me,
  on,
  onMe,
}: {
  me: { face: Avatar | null; paused: boolean } | null;
  /** Whether 这台机器 is the page on screen. */
  on: boolean;
  onMe: () => void;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          data-slot="button"
          data-rail-mark
          data-on={on}
          data-paused={me?.paused ? "" : undefined}
          aria-label={gui.identity}
          aria-current={on ? "page" : undefined}
          onClick={onMe}
          className="grid size-glass-btn flex-none place-items-center rounded-full hover:bg-fill data-[on=true]:bg-fill-strong focus-visible:outline-2 focus-visible:outline-ring"
        >
          <MachineAvatar face={me?.face ?? null} className="size-me-face" />
        </button>
      </TooltipTrigger>
      <TooltipContent side="bottom">{gui.identity}</TooltipContent>
    </Tooltip>
  );
}
