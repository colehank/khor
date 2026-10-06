/**
 * **A way back above a seat's list, inside the 附栏** (UX-39 §3): the
 * 附栏's one head (`SideBody`) says whose it is and which seat; a list
 * opened from a row of 最近 — today only a directory, the 工作区 tree —
 * wears ‹ back to 最近 here, a plain line under the 附栏's head: ‹ as a
 * glass circle, the list's name.
 *
 * ⚠️ 墓碑 (UX-37 §6 → UX-40 §4): a file's or a web page's head — ‹, the
 * name, the file's ✓ / preview / 「…」 — at the page's top. A page opened
 * in the 附栏 wears the main area's deck at the foot instead (`FilePage
 * docked`, the narrow `SessionScreen`). ⚠️ 墓碑 (UX-37 §1 → UX-39 §3): the
 * slice's head as ‹ to the whole and the whole's as its title alone
 * (「这个会话碰过的」 / 「全部最近」).
 */
import { GlassCircle } from "@/components/Glass";
import { IconBack } from "@/components/icons";

export function SeatHead({
  title,
  back,
  backLabel,
}: {
  title: string;
  /** To the list this one was opened from. */
  back: () => void;
  /** What ‹ goes to, for the ear: that list's name. */
  backLabel: string;
}) {
  return (
    <header data-seat-head className="flex min-h-ctl-lg flex-none items-center gap-row border-b ps-3 pe-(--page-inset)">
      <GlassCircle size="head" aria-label={backLabel} data-back data-seat-back onClick={back}>
        <IconBack />
      </GlassCircle>
      <span data-seat-title className="min-w-0 flex-1 truncate text-body font-label">
        {title}
      </span>
    </header>
  );
}
