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
 * dimmed by `--sheet-scrim` (darker in both themes; on dark, the
 * terminal's rejoin fade, UX-49 §1). ⚠️ 墓碑 (UX-37 §2 → UX-39 §1):
 * `popover` as the sheet's surface, `bg-black/50` as its scrim.
 * ⚠️ 墓碑 (UX-44 → UX-49): lighter on dark, a black terminal read as grey.
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
import {useContentSheet} from "@/hooks/use-content-sheet";
import * as DialogPrimitive from "@radix-ui/react-dialog";
import { useCallback, useState, type ReactNode } from "react";

import { DialogOverlay } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import type { SideSeat } from "@/hooks/use-session-side";
import { SideBody, type SideWho } from "@/views/SessionSide";

export type PlaceDetent = import("@/lib/overlay-policy").SheetDetent;

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
  useContentSheet(el, setDetent, close);
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
