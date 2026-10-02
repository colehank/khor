/**
 * **The wide face's navigation: the landings' glyphs in the window's top
 * row** (the redesign, 2026-10-02, the owner's 「方案 B」). One glass
 * capsule beside the window's buttons, the glyphs alone in the user's
 * order (`lib/nav-order` — all five; nothing folds behind 更多 here), the
 * open one a `fill-strong` pill with the brand's deep ink and its solid
 * piece filled (`rail-on-ink`, `app.css`). Its name is the hint a
 * pointer gets.
 *
 * **Pressing the open landing again folds its list away**, and pressing a
 * glyph brings it back; the shell does the folding. Folded, the strip
 * stays where it was on screen — the window's top-left — at the start of
 * the content's first row (`Region`'s lead).
 *
 * `MeButton` is the row's other end: this machine's face, opening a menu
 * of 设置 and 这台机器. The narrow face has neither: its navigation floats
 * (`FloatNav`).
 */
import type { ReactNode } from "react";

import { MachineAvatar } from "@/components/Avatar";
import { IconGear } from "@/components/icons";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { Avatar } from "@/gen/bindings/Avatar";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

export type StripSeat = { key: string; name: string; glyph: ReactNode; badge?: number };

export function LandingStrip({
  seats,
  active,
  onGo,
}: {
  seats: StripSeat[];
  /** The open landing, or null while a 「…」 page is up. */
  active: string | null;
  onGo: (key: string) => void;
}) {
  return (
    <nav
      data-landing-strip
      aria-label={gui.nav}
      className="glass flex h-glass-btn flex-none items-center gap-px rounded-full px-glass-pad"
    >
      {seats.map((s) => {
        const on = active === s.key;
        return (
          <Tooltip key={s.key}>
            <TooltipTrigger asChild>
              <button
                type="button"
                data-slot="button"
                data-rail-item
                data-landing={s.key}
                data-on={on}
                aria-label={s.name}
                aria-current={on ? "page" : undefined}
                onClick={() => onGo(s.key)}
                className={cn(
                  "relative grid h-glass-item-h w-strip-item place-items-center rounded-full text-rail-ink hover:bg-fill",
                  "focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring [&>svg]:size-head-glyph",
                  on && "bg-fill-strong text-[var(--rail-on-ink)] hover:bg-fill-strong",
                )}
              >
                {s.glyph}
                {s.badge !== undefined && s.badge > 0 && (
                  <span className="absolute -top-1 -right-1 min-w-4 rounded-full bg-badge px-1 text-center text-caption leading-4 tabular-nums text-badge-foreground">
                    {s.badge}
                  </span>
                )}
              </button>
            </TooltipTrigger>
            <TooltipContent side="bottom" data-rail-tip>
              {s.name}
            </TooltipContent>
          </Tooltip>
        );
      })}
    </nav>
  );
}

export function MeButton({
  me,
  on,
  onSettings,
  onMe,
}: {
  me: { face: Avatar | null; name: string; paused: boolean } | null;
  /** Whether 设置 or 这台机器 is the page on screen. */
  on: boolean;
  onSettings: () => void;
  onMe: () => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          data-slot="button"
          data-rail-mark
          data-on={on}
          data-paused={me?.paused ? "" : undefined}
          aria-label={gui.me_menu}
          className="grid size-glass-btn flex-none place-items-center rounded-full hover:bg-fill data-[on=true]:bg-fill-strong focus-visible:outline-2 focus-visible:outline-ring"
        >
          <MachineAvatar face={me?.face ?? null} className="size-me-face" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" onCloseAutoFocus={(e) => e.preventDefault()}>
        <DropdownMenuItem data-nav-settings onSelect={onSettings}>
          <IconGear className="size-4" />
          {gui.nav_settings}
        </DropdownMenuItem>
        <DropdownMenuItem data-nav-me onSelect={onMe}>
          <MachineAvatar face={me?.face ?? null} className="size-4" />
          {me?.name ?? gui.identity}
          <span className="ms-auto text-muted-foreground">{gui.identity}</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
