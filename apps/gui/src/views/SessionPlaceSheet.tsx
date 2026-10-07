/**
 * **The phone's pill opens the session's 附栏 as a sheet** (UX-30 §6; it
 * was UX-17's 目录 · 日志 sheet). The same five seats the wide face docks
 * beside the terminal (`SideBody`), risen from the bottom: 最近 · 工作区
 * · 浏览器 · 机器 · 额度, chosen from the head's title (`SeatTitle`); 目录
 * is the 工作区 seat. The seat is the session's own memory
 * (`use-session-side`); the day is 最近's first row (`DayToday`).
 *
 * **One sheet of paper** (UX-39 §1, the owner 2026-10-06 21:10: 「展开
 * 了就是 app 的颜色，不要额外颜色」): the app's own ground, a top rim and
 * the sheet's corners, the grabber inside the paper, and the page behind
 * greyed by `--sheet-scrim` (darker on light, lighter on dark — a black
 * terminal reads as grey). ⚠️ 墓碑 (UX-37 §2 → UX-39 §1): `popover` as
 * the sheet's surface, `bg-black/50` as its scrim.
 *
 * **Two detents** (UX-37 §3, the owner 2026-10-06, agreed on
 * `audit/session-sheet-demo.html`): the pill opens
 * it at half the screen, so the terminal keeps its upper half; the
 * grabber, a page's head, or the body while nothing under the finger has
 * scrolled drags it to nearly the whole screen or back; let go under
 * half it goes away, as a tap on the scrim and Esc do. The pages keep
 * the safe area at their foot (`--nav-room`); the session's deck steps
 * away while it is up (`SessionScreen`'s `away`, UX-37 §4). The heights
 * are tokens (`--place-sheet-half`, `--place-sheet-full`).
 *
 * ⚠️ 墓碑 (UX-37 §4 → UX-39 §2): the five seats as one flat row at the
 * sheet's foot, on the screen's foot, wearing the safe area.
 * ⚠️ 墓碑 (UX-30 → UX-37): the sheet was the form dialog's sheet (`ui/
 * dialog` `form`) at 80 % of the screen, padded by the dialog's own
 * inset, the capsule of seats at its top.
 * ⚠️ 墓碑 (UX-30 §5 → UX-36): the day's heat line stood under the
 * capsule as a ruler, and a tap on it expanded the day.
 * ⚠️ 墓碑 (UX-17 → UX-30): a 目录 / 日志 segment with its own `keptTab`
 * store per session; the seat replaces the tab.
 */
import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";

import { DialogOverlay } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import type { SideSeat } from "@/hooks/use-session-side";
import { SideBody, type SideWho } from "@/views/SessionSide";

export type PlaceDetent = "half" | "full";

/** A detent as a share of the screen's height, read from its token; the
    fallbacks mirror tokens.css and only catch a token failing to resolve. */
function share(detent: PlaceDetent): number {
  const name = detent === "half" ? "--place-sheet-half" : "--place-sheet-full";
  try {
    const v = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name));
    if (Number.isFinite(v) && v > 0 && v <= 1) return v;
  } catch {
    /* the fallback below */
  }
  return detent === "half" ? 0.5 : 0.92;
}

/** How fast a release has to be moving, in px per ms, to count as a
    throw rather than a placement — the form sheet's own (`ui/dialog`). */
const THROW = 0.5;
/** Under this share of the half detent a release puts the sheet away
    (the prototype's `HALF * 0.55`): a pull that ends a little under half
    springs back, one that has gone most of the way down is a dismissal. */
const AWAY = 0.55;

/**
 * The drag between the detents. Touch, not pointer, events, for the
 * form sheet's reason (`ui/dialog` `useSheet`): a pointer on a scrolling
 * page is taken by the browser's pan before it has moved far, and a touch
 * move that is prevented is the sheet's. **Whose the touch is**: at once
 * on the grabber; elsewhere once it has moved up or down rather than
 * sideways (a row's swipe keeps its own), and only while nothing between
 * the finger and the sheet has scrolled — a list scrolled down scrolls
 * back first. At the full detent a move up is the page's scroll. A
 * slider or anything marked `data-sheet-hold` keeps its touches.
 */
function useDetents(el: HTMLDivElement | null, onDetent: (d: PlaceDetent) => void, onClose: () => void) {
  // Read through a ref: the shell hands a fresh `onOpenChange` on every
  // render (a poll lands every two seconds), and re-binding the listeners
  // mid-drag would drop the pull under the finger.
  const told = useRef({ onDetent, onClose });
  told.current = { onDetent, onClose };
  useEffect(() => {
    if (!el) return;
    const close = () => told.current.onClose();
    const detent = (d: PlaceDetent) => told.current.onDetent(d);
    let from: number | null = null;
    let fromX = 0;
    let fromH = 0;
    let pulling = false;
    let live = 0;
    let speed = 0;
    let lastY = 0;
    let lastT = 0;
    const scrolled = (t: Element | null) => {
      for (let n = t; n && n !== el; n = n.parentElement) {
        if (n instanceof HTMLElement && n.scrollTop > 0) return true;
      }
      return false;
    };
    const grip = () => {
      pulling = true;
      el.setAttribute("data-sheet-pulling", "");
    };
    const down = (e: TouchEvent) => {
      if (e.touches.length !== 1) return;
      const t = e.target instanceof Element ? e.target : null;
      const band = !!t?.closest("[data-place-grab]");
      if (!band && (t?.closest('[data-sheet-hold], input[type="range"]') || scrolled(t))) return;
      from = e.touches[0].clientY;
      fromX = e.touches[0].clientX;
      fromH = el.getBoundingClientRect().height;
      live = fromH;
      lastY = from;
      lastT = e.timeStamp;
      speed = 0;
      pulling = false;
      if (band) grip();
    };
    const move = (e: TouchEvent) => {
      if (from === null) return;
      const y = e.touches[0].clientY;
      const dy = y - from;
      if (!pulling) {
        const dx = Math.abs(e.touches[0].clientX - fromX);
        if (Math.max(Math.abs(dy), dx) < 6) return;
        if (Math.abs(dy) <= dx || (dy < 0 && el.getAttribute("data-detent") === "full")) {
          from = null;
          return;
        }
        grip();
      }
      e.preventDefault();
      live = Math.max(0, Math.min(fromH - dy, share("full") * window.innerHeight));
      el.style.height = `${live}px`;
      const dt = e.timeStamp - lastT;
      if (dt > 0) speed = (y - lastY) / dt;
      lastY = y;
      lastT = e.timeStamp;
    };
    const up = () => {
      if (from === null) return;
      from = null;
      if (!pulling) return;
      pulling = false;
      el.removeAttribute("data-sheet-pulling");
      el.style.removeProperty("height");
      const half = share("half") * window.innerHeight;
      const full = share("full") * window.innerHeight;
      // A throw goes one detent its way: up to full, down to half from
      // above it, away from half or under; a placement takes the nearer.
      if (live < half * AWAY || (speed > THROW && live <= half)) return close();
      if (speed < -THROW) return detent("full");
      if (speed > THROW) return detent("half");
      detent(live > (half + full) / 2 ? "full" : "half");
    };
    el.addEventListener("touchstart", down, { passive: true });
    el.addEventListener("touchmove", move, { passive: false });
    el.addEventListener("touchend", up);
    el.addEventListener("touchcancel", up);
    return () => {
      el.removeEventListener("touchstart", down);
      el.removeEventListener("touchmove", move);
      el.removeEventListener("touchend", up);
      el.removeEventListener("touchcancel", up);
    };
  }, [el]);
}

export function SessionPlaceSheet({
  open,
  onOpenChange,
  seat,
  onSeat,
  who,
  boxRef,
  children,
}: {
  open: boolean;
  /** The box the day's axis is measured in (`useStripWidth`). */
  boxRef?: (el: HTMLDivElement | null) => void;
  onOpenChange: (open: boolean) => void;
  seat: SideSeat;
  onSeat: (seat: SideSeat) => void;
  who: SideWho;
  /** The seat's page, composed by the shell (`App`'s `sideSeat`). */
  children?: ReactNode;
}) {
  // Each opening starts at half (§3): the pill's tap is a glance. The
  // place it opens on is where it was left (UX-47); the height is not.
  const [detent, setDetent] = useState<PlaceDetent>("half");
  const [el, setEl] = useState<HTMLDivElement | null>(null);
  const close = useCallback(() => onOpenChange(false), [onOpenChange]);
  useDetents(el, setDetent, close);
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        {/* The page behind turns grey (UX-39 §1): the one scrim every
            sheet and dialog wears (UX-44 §2), `DialogOverlay`'s own. */}
        <DialogOverlay />
        <DialogPrimitive.Content
          ref={setEl}
          data-place-sheet
          data-detent={detent}
          // **A find row's box or a page's address field focused** (UX-40
          // §2): the sheet rises whole, as Maps' does, so the keyboard does
          // not sit on the answers.
          onFocusCapture={(e) => {
            if ((e.target as HTMLElement).closest("[data-side-find], [data-page-address]")) setDetent("full");
          }}
          // The head and the page are the sheet's whole content; a
          // description would be the decorative copy `ui/dialog` opts out of.
          aria-describedby={undefined}
          // **The focus goes to the sheet itself**, not to its first
          // control: the seat's page mounts its rows after the sheet does,
          // and a first control that was replaced left the focus on the
          // terminal under the sheet, which took Esc for the shell (a
          // keyboard's Esc, and the soft keyboard's lowering, are the
          // sheet's while it is up).
          onOpenAutoFocus={(e) => {
            e.preventDefault();
            el?.focus({ preventScroll: true });
          }}
          // The paper (ground, rim, corners, shadow) is every sheet's, written
          // once in `app.css` (UX-44 §1).
          className="fixed inset-x-0 bottom-0 z-50 flex flex-col overflow-hidden outline-none"
        >
          <DialogPrimitive.Title className="sr-only">{gui.session_side}</DialogPrimitive.Title>
          {/* The grabber's band: where a pull is the sheet's at once. */}
          <div data-place-grab aria-hidden="true" className="flex h-[calc(2*var(--grabber-top)+var(--grabber-h))] flex-none justify-center pt-(--grabber-top)">
            <span className="h-(--grabber-h) w-(--grabber-w) rounded-full bg-tertiary" />
          </div>
          <div
            ref={boxRef}
            data-place-body
            // The pages read the sheet's ground as theirs (`--side-ground`),
            // and their scrollers keep the safe area at their foot
            // (`data-nav-shown`, `--nav-room` in `app.css`).
            data-nav-shown
            className="relative flex min-h-0 flex-1 flex-col [--side-ground:var(--background)]"
          >
            <SideBody seat={seat} onSeat={onSeat} who={who}>
              {children}
            </SideBody>
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
