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
import { useRef, type ReactNode } from "react";

import { GlassCircle } from "@/components/Glass";
import { IconMore } from "@/components/icons";
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { gui } from "@/gen/catalog";
import { useTravel } from "@/lib/travel";
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
            "relative grid h-strip-item-h w-strip-item place-items-center rounded-full text-rail-ink",
            "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
            on && "glass-sel text-[var(--rail-on-ink)]",
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
  moreMenu,
}: {
  /** The four landings on the bar (`NAV_SHOWN`), as the phone's. */
  seats: StripSeat[];
  /** The open landing, or null while a 「…」 page is up. */
  active: string | null;
  /** Whether 更多 holds the page on screen. */
  moreOn: boolean;
  onGo: (key: string) => void;
  /** 更多's menu: 我的 khor, the folded landings, 设置, 诊断 (`MoreMenuItems`). */
  moreMenu: ReactNode;
}) {
  // The chosen pill slides to the landing pressed (F7.2, `lib/travel`).
  const strip = useRef<HTMLElement>(null);
  useTravel("strip", () => strip.current, active ?? (moreOn ? "more" : null));
  return (
    // **Four in the capsule, 更多 a circle of its own** (UX-39, the owner
    // 2026-10-06 23:00: 「更多和其他分开，更多和下面的搜索按钮对齐」; the
    // 2026-10-02 arrangement again): the four landings share the glass
    // capsule, and 更多 stands after it as a glass circle on the search
    // circle's column below. The phone's bar keeps all five in one
    // (`FloatNav`). ⚠️ 墓碑 (UX-19): 更多 as the capsule's fifth seat.
    <div data-landing-pair className="flex min-w-0 flex-1 items-center gap-in">
      <nav
        ref={strip}
        data-landing-strip
        aria-label={gui.nav}
        className="glass flex h-strip-h min-w-0 flex-1 items-center justify-between rounded-full px-glass-pad"
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
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <GlassCircle
            data-rail-item
            data-rail-more
            data-on={moreOn}
            aria-label={gui.rail_more}
            className={cn("data-[state=open]:glass-sel", moreOn && "glass-sel text-[var(--rail-on-ink)]")}
          >
            <IconMore />
          </GlassCircle>
        </DropdownMenuTrigger>
        <DropdownMenuContent
          side="bottom"
          align="end"
          sideOffset={8}
          data-nav-menu
          data-rail-more-menu
          onCloseAutoFocus={(e) => e.preventDefault()}
          className="min-w-nav-menu"
        >
          {moreMenu}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
