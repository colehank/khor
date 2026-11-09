/**
 * **The floating navigation, narrow only** (the redesign, 2026-10-02:
 * Apple's Preview on a phone). One glass capsule: the first four
 * landings in the user's order (`lib/nav-order`, four since UX-11), then
 * 更多 — floating,
 * centred, over the foot of the screen. The wide face
 * has the landings' strip in its top row instead (`LandingStrip`).
 *
 * The overflow chooser opens hidden landings or the separate More page.
 *
 * The open seat is the ground showing through the glass (`glass-sel`)
 * with the brand's deep ink, its glyph's solid piece filled (`rail-on-ink`,
 * `app.css`); the others are ink on the glass.
 */
import { useRef, type ReactNode } from "react";
import { IconMore } from "@/components/icons";
import {NavigationOverflow} from "@/components/NavigationOverflow";
import { useTravel } from "@/lib/travel";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

export type NavSeat = { key: string; name: string; glyph: ReactNode; badge?: number };

// A seat is `--nav-item` wide and gives way when five do not fit the
// screen (UX-11): the bar never runs past the screen's gaps. ⚠️ 墓碑
// (UX-30 §2 → UX-39 §2): exported for the 附栏's capsule of seats, which
// was this bar at the top of its box.
const NAV_SEAT =
  "relative flex h-full w-nav-item min-w-0 shrink flex-col items-center justify-center gap-px rounded-full text-foreground " +
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
      className={NAV_SEAT}
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

/** More is a destination alongside the four primary landings. */
export function FloatNav({seats,more,active,moreOn,onGo,onMore,onMe,unreadNotifications=0,className}: {
  seats:NavSeat[];
  more:NavSeat[];
  active:string|null;
  moreOn:boolean;
  onGo:(key:string)=>void;
  onMore:()=>void;
  onMe:()=>void;
  unreadNotifications?:number;
  className?:string;
}) {
  // The chosen seat slides to the one pressed (F7.2, `lib/travel`).
  const bar = useRef<HTMLElement>(null);
  useTravel("bar", () => bar.current, active ?? (moreOn ? "more" : null));
  return (
    // **Five seats in one capsule** (the owner, round 14: on the narrow
    // bar 更多 is not a button of its own — ⚠️ 墓碑, round 8 split it into
    // a circle beside the capsule; the wide strip keeps that). The four
    // landings (UX-11), then 更多 with its word under the glyph.
    <nav
      ref={bar}
      data-float-nav
      aria-label={gui.nav}
      // The padding keeps the hairline the glass had before its edge
      // became a ring (round 7), so the capsule is still 314 wide.
      className={cn(
        "glass flex h-nav-h max-w-[calc(100%-2*var(--nav-gap))] items-stretch rounded-full p-[calc(var(--gap-xxs)+var(--hair))] select-none",
        className,
      )}
    >
      {seats.map((s) => (
        <Seat key={s.key} seat={s} on={active === s.key} onClick={() => onGo(s.key)} />
      ))}
      <NavigationOverflow seats={more} onGo={onGo} onMore={onMore} onMe={onMe} mobile><button
        type="button" data-slot="button" data-nav-more data-on={moreOn}
        aria-label={gui.rail_more} aria-current={moreOn ? "page" : undefined}
        className={cn(NAV_SEAT,"data-[state=open]:glass-sel")}
      >
        <IconMore />
        <span className="text-nav-label">{gui.rail_more}</span>
        {unreadNotifications > 0 && <span data-nav-badge className="absolute top-1 right-3 min-w-4 rounded-full bg-badge px-1 text-center text-caption leading-4 tabular-nums text-badge-foreground">{unreadNotifications}</span>}
      </button></NavigationOverflow>
    </nav>
  );
}
