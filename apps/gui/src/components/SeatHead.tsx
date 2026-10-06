/**
 * **A page's head inside the 附栏** (UX-39 §2–3): the 附栏's one head
 * (`SideBody`) says whose it is and which seat; a seat's page draws a
 * head of its own only to carry a way back inside the seat — a file
 * opened from the tree back to the tree, a page opened from a row of 最近
 * back to 最近, a page of 浏览器 back to its list — or acts that belong
 * to the page (the file page's ✓, preview, 「…」). Then it is a plain
 * line under the 附栏's head: ‹ as a glass circle, the page's name. With
 * neither it draws nothing; the seat's noun is already said above.
 *
 * ⚠️ 墓碑 (UX-37 §1 → UX-39 §3): the slice's head as ‹ to the whole and
 * the whole's as its title alone (「这个会话碰过的」 / 「全部最近」).
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
  /** To the list this page was opened from. */
  back?: () => void;
  /** What ‹ goes to, for the ear: that list's name. */
  backLabel?: string;
  /** Anything that belongs at the head's right end. */
  children?: ReactNode;
}) {
  if (!back && !children) return null;
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
      <span data-seat-title className="min-w-0 flex-1 truncate text-body font-label">
        {title}
      </span>
      {children}
    </header>
  );
}
