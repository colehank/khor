/**
 * **The wide face's navigation: the landings' glyphs in the window's top
 * row** (the redesign, 2026-10-02, the owner's 「方案 B」). One glass
 * capsule beside the window's buttons — **the largest control in that
 * row** (the owner: it read smaller than the session tabs beside it), 36
 * high with 36 × 30 pills and `--ctl-glyph` glyphs — holding the landings in the
 * user's order (`lib/nav-order`, all five); 更多 is a 36 glass circle of
 * its own beside it.
 * The open one is a `fill-strong` pill with the brand's deep ink and its
 * solid piece filled (`rail-on-ink`, `app.css`); 更多 is lit the same way
 * while its pages are open. A glyph's name is the hint a pointer gets.
 *
 * **Pressing the open landing again folds its list away** — 更多 too,
 * while its pages are open — and once more brings it back; another glyph
 * switches and brings it back. The shell
 * does the folding. Folded, the strip stays where it was on screen — the
 * window's top-left — at the start of the content's first row
 * (`Region`'s lead).
 *
 * 这台机器 is inside 更多 (the owner, 2026-10-02: its face left the top
 * row). The narrow face has no strip: its navigation floats (`FloatNav`).
 */
import type { ReactNode } from "react";

import { IconMore } from "@/components/icons";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

export type StripSeat = { key: string; name: string; glyph: ReactNode; badge?: number };

function Seat({
  label,
  on,
  mark,
  onClick,
  round = false,
  children,
}: {
  label: string;
  on: boolean;
  mark: Record<string, string>;
  onClick: () => void;
  /** 更多's: a round as tall as a pill, not a pill. */
  round?: boolean;
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
            "relative grid h-strip-item-h place-items-center rounded-full text-rail-ink hover:bg-fill",
            round ? "w-strip-item-h" : "w-strip-item",
            "focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring",
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
    // The landings in one glass capsule, and 更多 a glass circle of its own
    // beside it (the owner, 2026-10-02: 「单独一个按钮，和别的四个分开」) —
    // the list head's grammar, a capsule and a circle.
    <div data-landing-pair className="flex flex-none items-center gap-in">
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
      </nav>
      <div className="glass grid size-strip-h flex-none place-items-center rounded-full">
        <Seat label={gui.rail_more} on={moreOn} mark={{ "data-rail-more": "" }} onClick={onMore} round>
          <IconMore />
        </Seat>
      </div>
    </div>
  );
}
