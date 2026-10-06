/**
 * **The phone's pill opens the session's 附栏 as a sheet** (UX-30 §6; it
 * was UX-17's 目录 · 日志 sheet). The same five seats the wide face docks
 * beside the terminal (`SideBody`), risen from the bottom: 最近 · 工作区
 * · 浏览器 · 机器 · 额度; 目录 is the 工作区 seat. Under the capsule the
 * 「今天」 row (UX-36 §1): the day's line hangs under the pill, whose tap
 * is this sheet, so the day in full opens from here. The seat and the
 * day's expansion are the session's own memory (`use-session-side`).
 *
 * ⚠️ 墓碑 (UX-30 §5 → UX-36): the day's heat line stood under the
 * capsule as a ruler, and a tap on it expanded the day.
 * ⚠️ 墓碑 (UX-17 → UX-30): a 目录 / 日志 segment with its own `keptTab`
 * store per session; the seat replaces the tab.
 */
import type { ReactNode } from "react";

import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import type { SideSeat } from "@/hooks/use-session-side";
import { SideBody } from "@/views/SessionSide";

export function SessionPlaceSheet({
  open,
  onOpenChange,
  seat,
  onSeat,
  today,
  boxRef,
  children,
}: {
  open: boolean;
  /** The box the day's axis is measured in (`useStripWidth`). */
  boxRef?: (el: HTMLDivElement | null) => void;
  onOpenChange: (open: boolean) => void;
  seat: SideSeat;
  onSeat: (seat: SideSeat) => void;
  /** The 「今天」 row (`DayToday`). */
  today?: ReactNode;
  /** The seat's page, composed by the shell (`App`'s `sideSeat`). */
  children?: ReactNode;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent form data-place-sheet className="flex h-[80dvh] flex-col !gap-0 overflow-hidden">
        <DialogTitle className="sr-only">{gui.session_side}</DialogTitle>
        <div ref={boxRef} data-place-body className="flex min-h-0 flex-1 flex-col [--side-ground:var(--popover)]">
          <SideBody seat={seat} onSeat={onSeat} today={today}>
            {children}
          </SideBody>
        </div>
      </DialogContent>
    </Dialog>
  );
}
