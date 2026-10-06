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

import { GlassCircle } from "@/components/Glass";
import { IconBack } from "@/components/icons";
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
      <span data-seat-title className="min-w-0 flex-1 truncate text-title-md font-title">
        {title}
      </span>
      {children}
    </header>
  );
}
