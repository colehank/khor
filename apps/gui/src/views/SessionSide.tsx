/**
 * **The session's 附栏: a phone docked beside the terminal** (UX-30, the
 * owner 2026-10-06:「一边 vibe 一边看交付物，在一个屏内不用切换」).
 *
 * A column (`--session-side-w`, 640: sb-a) beside a terminal or
 * a conversation on the wide face, running the narrow face's own pages:
 * it is `NarrowBox` (`hooks/use-narrow`), so everything inside lays
 * itself out as on the phone — list ⇄ detail, the same memory rules —
 * and nothing in here is designed twice. It stands beside the tab strip,
 * from the window's first row (UX-39 §6), on the app's own ground. At its
 * top one head (`SideBody`): the session's face and title at the left,
 * the seat bar at the right — 最近 · 工作区 · 浏览器 · 更多, 更多 holding
 * 机器 and 额度 (UX-45, `SeatBar`) — and under it the seat's page. The seat's content is composed by the shell (`App`), which
 * owns the state those pages read; this file is the box, the head, and
 * the seam.
 *
 * ⚠️ 墓碑 (UX-30 §2 → UX-39 §2): a glass capsule of the five seats at the
 * column's top. ⚠️ 墓碑 (UX-30 §5 → UX-36 → UX-39 §4): the session's day
 * as a ruler under the capsule, then as a line under the pill; it is
 * 最近's first row now (`views/DayToday`). ⚠️ 墓碑 (UX-30 → UX-39 §6):
 * the column under the strip, inside `DetailPane`.
 *
 * **The seam** is the column's left edge: dragged, the column takes
 * width from the session beside it (the terminal refits as it goes);
 * under the floor it folds the column away, the same gesture the list
 * pane's seam makes (`use-list-width`); double-click is the default
 * width; ← → Home walk the stops from a keyboard. The width is one
 * memory per seat (sb-a: 浏览器, 工作区, 最近 each its own), and open one
 * per session: each pill opens or closes its own as it comes to the front
 * (fix-e, `use-session-side`).
 *
 * **It gives way first** (sb-a, 2026-10-09, the owner: 「附栏打开时, 缩小
 * 窗口不能把整个顶部导航给略缩畸形了」): its width is a flex basis that
 * shrinks ahead of the session beside it (`App`'s `sideMain`), down to
 * `--session-side-min`; only then does the session narrow.
 */
import { useMemo, useRef, type PointerEvent as ReactPointerEvent, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";

import type { Avatar } from "@/gen/bindings/Avatar";
import { SeatBar, SeatPick } from "@/components/SeatTitle";
import { SessionFace } from "@/components/SessionFace";
import { SideSeatsProvider } from "@/components/SideHead";
import { gui } from "@/gen/catalog";
import { NarrowBox } from "@/hooks/use-narrow";
import { WINDOW_DRAG } from "@/components/shell";
import { type SideSeat } from "@/hooks/use-session-side";
import { band } from "@/lib/scroll-edge";

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
  const min = tokenPx("--session-side-min", 360);
  return {
    def: tokenPx("--session-side-w", 640),
    min,
    // Under the snap line a drag stops meaning「this wide」and starts
    // meaning「away」 — the list seam's own rule, half its snap band.
    snap: min - tokenPx("--list-w-snap", 100) / 2,
    cap: Math.max(min, Math.floor(room / 2)),
  };
}

export function SessionSide({
  session,
  width,
  seat,
  onSeat,
  onWidth,
  onLive,
  onClose,
  who,
  boxRef,
  children,
  embedded = false,
  maximized = false,
}: {
  session: string;
  embedded?: boolean;
  maximized?: boolean;
  /** The box the day's band is measured in (`useStripWidth`): its row
      is 最近's first (UX-39 §4), in this column. */
  boxRef?: (el: HTMLDivElement | null) => void;
  /** Whose column: the session's face and title in the head. */
  who: SideWho;
  /** The width it was dragged to, or `null` for the token's default. */
  width: number | null;
  seat: SideSeat;
  onSeat: (seat: SideSeat) => void;
  /** The seam let go at this width (`null` = back to the default). */
  onWidth: (w: number | null) => void;
  /** The seam moving: the session beside it gives or takes the same
      pixels in the same frame (`App`). */
  onLive?: (w: number | null) => void;
  /** The seam dragged under the snap line: the column goes away. */
  onClose: () => void;
  /** The seat's page, composed by the shell. */
  children?: ReactNode;
}) {
  const box = useRef<HTMLDivElement | null>(null);
  const setBox = (el: HTMLDivElement | null) => {
    box.current = el;
    boxRef?.(el);
  };

  // The drag paints through the box's own style each move — a poll-driven
  // re-render mid-drag would repaint a stale width over the live one —
  // and tells the store once the pointer lifts.
  const drag = useRef<{ right: number; live: number | null; away: boolean } | null>(null);
  const paint = (w: number | null) => {
    if (box.current) box.current.style.flexBasis = w === null ? "var(--session-side-w)" : `${w}px`;
    onLive?.(w);
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
      className="relative flex h-full min-h-0 flex-col border-l bg-card [--side-ground:var(--card)]"
      // Keep the main pane's shrink factor at one. A fractional factor
      // leaves overflow after this column reaches its minimum; weighting
      // this column instead still makes it give way first.
      style={{ flex: maximized ? "1 1 100%" : `0 ${embedded ? 1000 : 1} ${width === null ? "var(--session-side-w)" : `${width}px`}`, minWidth: embedded ? "min(var(--session-side-min), 50%)" : "min(var(--session-side-min), 100%)" }}
    >
      {/* The seam: a hit margin overhanging the border, taking no room. */}
      <div
        hidden={maximized}
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
      {embedded ? children : <SideBody seat={seat} onSeat={onSeat} who={who}>
        {children}
      </SideBody>}
    </NarrowBox>
  );
}

/** Whose 附栏 this is: the session's face and title, as its list row
    draws them (`SessionFace`). */
export type SideWho = { face: Avatar | null; title: string; word: string | null; kind: string; category: string | null };

/**
 * The 附栏's inside — the one head and the seat's page — which is also
 * the phone's pill sheet (UX-30 §6: both faces share the components;
 * `SessionPlaceSheet` mounts this in a bottom sheet).
 */
export function SideBody({
  seat,
  onSeat,
  who,
  children,
}: {
  seat: SideSeat;
  onSeat: (seat: SideSeat) => void;
  who: SideWho;
  children?: ReactNode;
}) {
  // **One head** (UX-39 §2, the owner 2026-10-06 21:40: 「左上角是
  // session 的头像+标题，右上角是可展开的标题」): the session at the left
  // — its face and title, the subject the 附栏 belongs to — and the
  // seat bar at the right (UX-45, `SeatBar`: 最近 · 工作区 · 浏览器 · 更多,
  // 更多 holding 机器 and 额度). The seats' own heads step back inside
  // (`SeatHead` draws only the tree's way back to 最近; a page wears the
  // main area's deck at its foot, UX-40 §4). ⚠️ 墓碑: the glass capsule
  // at the column's top (UX-30 §2), the flat row at the sheet's foot
  // (UX-37 §4), the page's own title at the left (「这个会话碰过的」, the
  // machine's name), the seat's icon · noun · ⌄ over a native picker
  // (UX-39 §2 → UX-45, `SeatTitle`).
  const pick = useMemo(() => ({ seat, onSeat }), [seat, onSeat]);
  return (
    <SeatPick.Provider value={pick}>
      <SideSeatsProvider value={null}>
        {/* **The column's first row moves the window** (the owner
            2026-10-07 15:50: 「desktop 端点击药丸展开的侧边，无法在这个标题
            上拖动」): on the Mac it stands beside R0, top-aligned, and is a
            first row like it — the shell's one mark (`WINDOW_DRAG`); the
            seat control keeps its press (tauri's own rule). */}
        <header ref={band} data-band data-side-head {...WINDOW_DRAG} className="flex h-pane-head flex-none items-center gap-row border-b px-(--page-inset)">
          <span data-side-who className="flex min-w-0 flex-1 items-center gap-in">
            <SessionFace face={who.face} word={who.word} kind={who.kind} category={who.category} size="tab" />
            <span className="min-w-0 truncate text-title-md font-title">{who.title}</span>
          </span>
          <SeatBar />
        </header>
        <div data-side-page className="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
          {children}
        </div>
      </SideSeatsProvider>
    </SeatPick.Provider>
  );
}
