/**
 * **A 附栏 seat's head** (UX-37 §1): every seat lands on this session's
 * slice, and ‹ goes to the whole. The slice's head is ‹ · its title; the
 * whole's is its title alone (a row in it lands back on a slice). The
 * same strip the narrow detail pages draw where there is no window row
 * to seat a head in (`MachineCard`'s, 额度's): the way back as a glass
 * circle, then the name — so a seat's own head and a card's read as one
 * kind of thing inside the sheet.
 */
import type { ReactNode } from "react";

import { useContext } from "react";

import { GlassCircle } from "@/components/Glass";
import { IconBack } from "@/components/icons";
import { SeatPick, SeatTitle } from "@/components/SeatTitle";
import { cn } from "@/lib/utils";

export function SeatHead({
  title,
  back,
  backLabel,
  children,
}: {
  title: ReactNode;
  /** To the whole; absent on the whole itself. */
  back?: () => void;
  /** What ‹ goes to, for the ear: the whole's name. */
  backLabel?: string;
  /** Anything that belongs at the head's right end. */
  children?: ReactNode;
}) {
  // **Inside the 附栏 nothing goes back** (UX-39, the owner 2026-10-06
  // 21:05: 「都不可返回，全都和 额度 一样」): a seat is one page under its
  // title; ‹ is drawn only where the head stands on its own screen.
  const inSide = useContext(SeatPick) !== null;
  // The 附栏's one head is `SideBody`'s (UX-39): a seat's own head draws
  // nothing here unless it carries a way back inside the seat (a file
  // page to its tree, a page opened from 最近 back to 最近) or acts, and
  // then as a plain line under the head.
  if (inSide && !back && !children) return null;
  return (
    <header
      data-seat-head
      className={cn("flex min-h-ctl-lg flex-none items-center gap-row border-b pe-(--page-inset)", back ? "ps-3" : "ps-(--page-inset)")}
    >
      {back && (
        <GlassCircle size="head" aria-label={backLabel} data-back data-seat-back onClick={back}>
          <IconBack />
        </GlassCircle>
      )}
      {inSide ? (
        <span data-seat-title className="min-w-0 flex-1 truncate text-body font-label">
          {title}
        </span>
      ) : (
        <SeatTitle className="flex-1">
          <span data-seat-title>{title}</span>
        </SeatTitle>
      )}
      {children}
    </header>
  );
}
