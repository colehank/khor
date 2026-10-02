/**
 * **The floating navigation, narrow only** (the redesign, 2026-10-02:
 * Apple's Preview on a phone). One glass capsule: the first three
 * landings in the user's order (`lib/nav-order`), then 更多 — floating,
 * centred, over the foot of the screen. The wide face
 * has the landings' strip in its top row instead (`LandingStrip`).
 *
 * **更多 opens a menu in place**, anchored above it: the landings past
 * the third, 设置, and — under a rule — this machine. Choosing one goes
 * there; 更多 is lit while any of them is the page on screen, because
 * the bar must always answer 我在哪一页.
 *
 * The open seat is the ground showing through the glass (`glass-sel`)
 * with the brand's deep ink, its glyph's solid piece filled (`rail-on-ink`,
 * `app.css`); the others are ink on the glass.
 */
import type { ReactNode } from "react";

import { MachineAvatar } from "@/components/Avatar";
import { IconGear, IconMore } from "@/components/icons";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { Avatar } from "@/gen/bindings/Avatar";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

export type NavSeat = { key: string; name: string; glyph: ReactNode; badge?: number };

const SEAT =
  "relative flex h-full w-nav-item flex-none flex-col items-center justify-center gap-px rounded-full text-foreground " +
  "focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring " +
  "[&_svg]:size-nav-glyph data-[on=true]:glass-sel data-[on=true]:text-[var(--rail-on-ink)]";

function Seat({ seat, on, onClick }: { seat: NavSeat; on: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      data-slot="button"
      data-nav-item={seat.key}
      data-landing={seat.key}
      data-on={on}
      aria-current={on ? "page" : undefined}
      onClick={onClick}
      className={SEAT}
    >
      {seat.glyph}
      {/* One weight in both states (F5.2; SYS-12): the chosen piece and the
          tint say which, and the word does not change width under it. */}
      <span className="text-nav-label">{seat.name}</span>
      {seat.badge !== undefined && seat.badge > 0 && (
        <span
          data-nav-badge
          className="absolute top-1 right-3 min-w-4 rounded-full bg-badge px-1 text-center text-caption leading-4 tabular-nums text-badge-foreground"
        >
          {seat.badge}
        </span>
      )}
    </button>
  );
}

/** The menu is the one menu (`ui/dropdown-menu`); its rows carry a nav glyph. */
const MENU_ITEM = "[&_svg]:size-nav-glyph";

export function FloatNav({
  seats,
  more,
  active,
  moreOn,
  onGo,
  onSettings,
  me,
  onMe,
  className,
}: {
  /** The three on the bar. */
  seats: NavSeat[];
  /** The landings past the third, at the head of 更多's menu. */
  more: NavSeat[];
  /** The open landing's key, or null while a 「…」 page is up. */
  active: string | null;
  /** Whether 更多 holds the page on screen. */
  moreOn: boolean;
  onGo: (key: string) => void;
  onSettings: () => void;
  /** This machine, at the foot of the menu: its face and name. */
  me: { face: Avatar | null; name: string } | null;
  onMe: () => void;
  className?: string;
}) {
  return (
    // **Four seats in one capsule** (the owner, round 14: on the narrow
    // bar 更多 is not a button of its own — ⚠️ 墓碑, round 8 split it into
    // a circle beside the capsule; the wide strip keeps that). The three
    // landings, then 更多 with its word under the glyph, 314 wide.
    <nav
      data-float-nav
      aria-label={gui.nav}
      // The padding keeps the hairline the glass had before its edge
      // became a ring (round 7), so the capsule is still 314 wide.
      className={cn("glass flex h-nav-h items-stretch rounded-full p-[calc(var(--gap-xxs)+var(--hair))] select-none", className)}
    >
      {seats.map((s) => (
        <Seat key={s.key} seat={s} on={active === s.key} onClick={() => onGo(s.key)} />
      ))}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          {/* Chosen (the glass's chosen piece) while its menu is open,
              and olive too while a page reached through it is the one on
              screen. */}
          <button
            type="button"
            data-slot="button"
            data-nav-more
            data-on={moreOn}
            aria-label={gui.rail_more}
            className={cn(SEAT, "data-[state=open]:glass-sel")}
          >
            <IconMore />
            <span className="text-nav-label">{gui.rail_more}</span>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent
          side="top"
          align="end"
          sideOffset={8}
          data-nav-menu
          // Choosing goes somewhere else; the focus does not come back to
          // 更多 and light it with a ring nobody asked for.
          onCloseAutoFocus={(e) => e.preventDefault()}
          className="min-w-nav-menu"
        >
          {more.map((s) => (
            <DropdownMenuItem
              key={s.key}
              data-more-landing={s.key}
              className={MENU_ITEM}
              onSelect={() => onGo(s.key)}
            >
              {s.glyph}
              {s.name}
            </DropdownMenuItem>
          ))}
          <DropdownMenuItem data-nav-settings className={MENU_ITEM} onSelect={onSettings}>
            <IconGear />
            {gui.nav_settings}
          </DropdownMenuItem>
          {me && (
            <>
              <DropdownMenuSeparator className="mx-row my-1.5" />
              <DropdownMenuItem data-nav-me className={cn(MENU_ITEM, "py-1.5")} onSelect={onMe}>
                <MachineAvatar face={me.face} className="size-ctl-md" />
                <span className="flex min-w-0 flex-col">
                  <span className="truncate">{me.name}</span>
                  <span className="text-callout text-muted-foreground">{gui.identity}</span>
                </span>
              </DropdownMenuItem>
            </>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
    </nav>
  );
}
