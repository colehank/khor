/**
 * **The session's 附栏: a phone docked beside the terminal** (UX-30, the
 * owner 2026-10-06:「一边 vibe 一边看交付物，在一个屏内不用切换」).
 *
 * A column at the phone's width (`--session-side-w`) beside a terminal or
 * a conversation on the wide face, running the narrow face's own pages:
 * it is `NarrowBox` (`hooks/use-narrow`), so everything inside lays
 * itself out as on the phone — list ⇄ detail, the same memory rules —
 * and nothing in here is designed twice. At its top a capsule of five
 * seats, 最近 · 工作区 · 浏览器 · 机器 · 额度, the phone's bar at the top
 * of the box, and under it the seat's page. (⚠️ 墓碑 UX-30 §5 → UX-36:
 * the session's day stood between them as a ruler; it hangs under the
 * session's pill now, `views/DayLine`.) The seat's content is composed by the
 * shell (`App`), which owns the state those pages read; this file is the
 * box, the capsule, and the seam.
 *
 * **The seam** is the column's left edge: dragged, the column takes
 * width from the session beside it (the terminal refits as it goes);
 * under the floor it folds the column away, the same gesture the list
 * pane's seam makes (`use-list-width`); double-click is the default
 * width; ← → Home walk the stops from a keyboard. Width and open state
 * are one memory per session (`use-session-side`).
 */
import { useRef, type PointerEvent as ReactPointerEvent, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";

import { IconBrowser, IconDevices, IconFiles, IconQuota, IconRecent } from "@/components/icons";
import { NAV_SEAT } from "@/components/FloatNav";
import { SideSeatsProvider } from "@/components/SideHead";
import { gui } from "@/gen/catalog";
import { NarrowBox } from "@/hooks/use-narrow";
import { SIDE_SEATS, type SideSeat } from "@/hooks/use-session-side";
import { useTravel } from "@/lib/travel";
import { cn } from "@/lib/utils";

/** The column, for whatever reaches it by name (the front tab's
    `aria-controls`, the gates). */
export const SIDE_ID = "khor-session-side";

// Numbers live in tokens.css; the fallbacks only catch a token failing
// to resolve at all, and mirror the values written there.
function tokenPx(name: string, fallback: number): number {
  try {
    const v = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name));
    return Number.isFinite(v) && v > 0 ? v : fallback;
  } catch {
    return fallback;
  }
}

/** The column's bounds, read at the gesture: the floor is the token's,
    the ceiling half of the main pane (the two-column answer). */
function bounds(box: HTMLElement | null) {
  const main = box?.closest<HTMLElement>("[data-main]") ?? null;
  const room = main ? main.getBoundingClientRect().width : window.innerWidth;
  const min = tokenPx("--session-side-min", 320);
  return {
    def: tokenPx("--session-side-w", 390),
    min,
    // Under the snap line a drag stops meaning「this wide」and starts
    // meaning「away」 — the list seam's own rule, half its snap band.
    snap: min - tokenPx("--list-w-snap", 100) / 2,
    cap: Math.max(min, Math.floor(room / 2)),
  };
}

const SEAT_GLYPH: Record<SideSeat, ReactNode> = {
  recent: <IconRecent />,
  workspace: <IconFiles />,
  browser: <IconBrowser />,
  machine: <IconDevices />,
  quota: <IconQuota />,
};

function seatName(seat: SideSeat): string {
  switch (seat) {
    case "recent":
      return gui.group_recent;
    case "workspace":
      return gui.workspaces_tab;
    case "browser":
      return gui.browser_tab;
    case "machine":
      return gui.devices_tab;
    case "quota":
      return gui.accounts_tab;
  }
}

export function SessionSide({
  session,
  width,
  seat,
  onSeat,
  onWidth,
  onClose,
  children,
}: {
  session: string;
  /** The width it was dragged to, or `null` for the token's default. */
  width: number | null;
  seat: SideSeat;
  onSeat: (seat: SideSeat) => void;
  /** The seam let go at this width (`null` = back to the default). */
  onWidth: (w: number | null) => void;
  /** The seam dragged under the snap line: the column goes away. */
  onClose: () => void;
  /** The seat's page, composed by the shell. */
  children?: ReactNode;
}) {
  const box = useRef<HTMLDivElement | null>(null);
  const setBox = (el: HTMLDivElement | null) => {
    box.current = el;
  };

  // The drag paints through the box's own style each move — a poll-driven
  // re-render mid-drag would repaint a stale width over the live one —
  // and tells the store once the pointer lifts.
  const drag = useRef<{ right: number; live: number | null; away: boolean } | null>(null);
  const paint = (w: number | null) => {
    if (box.current) box.current.style.width = w === null ? "" : `${w}px`;
  };
  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    // Measured from the column's own right edge, read off the DOM at the
    // grab: the handle's box overhangs the seam by its hit margin.
    const r = box.current?.getBoundingClientRect();
    drag.current = { right: r ? r.right : e.clientX, live: width, away: false };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  };
  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d) return;
    const b = bounds(box.current);
    const raw = d.right - e.clientX;
    if (raw < b.snap) {
      d.away = true;
      // The width of the grab is kept: the comeback restores it.
      paint(Math.max(b.min, Math.min(raw, b.cap)));
    } else {
      d.away = false;
      d.live = Math.min(Math.max(raw, b.min), b.cap);
      paint(d.live);
    }
  };
  const onPointerUp = () => {
    const d = drag.current;
    if (!d) return;
    drag.current = null;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    if (d.away) {
      paint(width);
      onClose();
    } else {
      onWidth(d.live);
    }
  };
  const onDoubleClick = () => {
    paint(null);
    onWidth(null);
  };
  /** The same seam from a keyboard: the stops are the floor, the default
      and the ceiling (`use-list-width`'s ladder); past the floor, away. */
  const onKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    const b = bounds(box.current);
    const now = width ?? b.def;
    const rungs = [b.min, b.def, b.cap].filter((w, i, all) => i === 0 || w > all[i - 1]);
    const at = rungs.findIndex((w) => w >= now - 1);
    // Wider is away from the column's own edge: ← grows a right-hand column.
    const dir = e.key === "ArrowLeft" ? 1 : e.key === "ArrowRight" ? -1 : e.key === "Home" ? 0 : null;
    if (dir === null) return;
    e.preventDefault();
    if (dir === 0) {
      paint(null);
      onWidth(null);
      return;
    }
    const to = (at < 0 ? rungs.length - 1 : at) + dir;
    if (to < 0) {
      onClose();
      return;
    }
    const w = rungs[Math.min(rungs.length - 1, to)];
    paint(w);
    onWidth(w);
  };

  return (
    <NarrowBox
      ref={setBox}
      id={SIDE_ID}
      data-session-side={session}
      data-col-seat-on={seat}
      aria-label={gui.session_side}
      className="relative flex h-full min-h-0 flex-none flex-col border-l bg-card [--side-ground:var(--card)]"
      style={{ width: width === null ? "var(--session-side-w)" : `${width}px` }}
    >
      {/* The seam: a hit margin overhanging the border, taking no room. */}
      <div
        role="separator"
        aria-orientation="vertical"
        aria-label={gui.side_seam}
        tabIndex={0}
        data-side-seam
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onDoubleClick={onDoubleClick}
        onKeyDown={onKeyDown}
        className="absolute inset-y-0 -left-1 z-10 w-2 cursor-col-resize outline-none transition-colors hover:bg-border focus-visible:bg-ring"
      />
      <SideBody seat={seat} onSeat={onSeat}>
        {children}
      </SideBody>
    </NarrowBox>
  );
}

/**
 * The column's inside — the capsule and the seat's page — which is also
 * the phone's pill sheet (UX-30 §6: both faces share the components;
 * `SessionPlaceSheet` mounts this in a bottom sheet, with its 「今天」 row
 * under the capsule, UX-36).
 */
export function SideBody({
  seat,
  onSeat,
  today,
  children,
}: {
  seat: SideSeat;
  onSeat: (seat: SideSeat) => void;
  /** The phone sheet's 「今天」 row (`DayToday`): where the day opens on
      the phone, whose pill's tap is the sheet's (UX-36 §1). */
  today?: ReactNode;
  children?: ReactNode;
}) {
  const capsule = useRef<HTMLElement>(null);
  // The chosen seat slides to the one pressed (F7.2, `lib/travel`).
  useTravel("side", () => capsule.current, seat);
  return (
    <>
      {/* **The phone's five seats, at the top of the box** (UX-30 §2):
          one glass capsule across the column, the width the phone's bar
          keeps off its edges. */}
      <div data-side-head className="flex flex-none justify-center px-nav-gap pt-in pb-xxs">
        <nav
          ref={capsule}
          data-col-seats
          aria-label={gui.session_side}
          className="glass flex h-nav-h max-w-full items-stretch rounded-full p-[calc(var(--gap-xxs)+var(--hair))] select-none"
        >
          {SIDE_SEATS.map((s) => (
            <button
              key={s}
              type="button"
              data-slot="button"
              data-col-seat={s}
              data-on={seat === s}
              aria-current={seat === s ? "page" : undefined}
              onClick={() => onSeat(s)}
              className={cn(NAV_SEAT)}
            >
              {SEAT_GLYPH[s]}
              <span className="text-nav-label">{seatName(s)}</span>
            </button>
          ))}
        </nav>
      </div>
      {today}
      {/* The seat's page: the narrow face's own, in a box. `relative`
          because the pages float their own navigation against the
          nearest placed ancestor, which must be this box and not the
          window. */}
      {/* No window row to seat a head in (`NarrowHead`): the pages draw
          their own `‹ title` bar in place, as the workspace column's
          session bar does. */}
      <SideSeatsProvider value={null}>
        <div data-side-page className="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">{children}</div>
      </SideSeatsProvider>
    </>
  );
}
