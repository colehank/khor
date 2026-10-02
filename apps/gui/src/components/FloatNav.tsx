/**
 * **The floating navigation, narrow only** (the redesign, 2026-10-02:
 * Apple's Preview on a phone). A glass capsule of the first three
 * landings in the user's order (`lib/nav-order`), and beside it, a glass
 * circle of its own, 更多 (the owner: 「单独一个按钮，和别的四个分开」) —
 * the pair floating, centred, over the foot of the screen. The wide face
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
      <span className={cn("text-nav-label", on && "font-title")}>{seat.name}</span>
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

const MENU_ITEM = "min-h-touch gap-row rounded-xl px-row text-body focus:glass-sel focus:text-foreground [&_svg]:size-nav-glyph";

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
    // **更多 is a button of its own** (the owner, 2026-10-02: 「单独一个按钮,
    // 和别的四个分开」) — the list head's grammar, a capsule and a circle
    // beside it: the landings in one glass capsule, 更多 a glass circle,
    // the pair centred together.
    <div data-float-nav-pair className={cn("flex items-center gap-nav-pair select-none", className)}>
    <nav
      data-float-nav
      aria-label={gui.nav}
      className="glass flex h-nav-h items-stretch rounded-full p-xxs"
    >
      {seats.map((s) => (
        <Seat key={s.key} seat={s} on={active === s.key} onClick={() => onGo(s.key)} />
      ))}
    </nav>
    <div className="glass grid size-nav-h flex-none place-items-center rounded-full">
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          {/* The glyph alone; its name is for the ear. Chosen (the
              ground showing through) while its menu is open, and olive
              too while a page reached through it is the one on screen. */}
          <button
            type="button"
            data-slot="button"
            data-nav-more
            data-on={moreOn}
            aria-label={gui.rail_more}
            className={cn(
              "relative grid size-nav-more-in place-items-center rounded-full text-foreground [&_svg]:size-nav-more-glyph",
              "focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring",
              "data-[on=true]:glass-sel data-[on=true]:text-[var(--rail-on-ink)] data-[state=open]:glass-sel",
            )}
          >
            <IconMore />
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
          className="glass min-w-nav-menu rounded-2xl border-0 bg-glass-menu p-1.5 shadow-[var(--glass-in),var(--glass-shadow)]"
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
    </div>
    </div>
  );
}
