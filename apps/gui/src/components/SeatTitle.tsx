/**
 * **The seat's title is the menu** (UX-39 §2, the owner 2026-10-06 20:30:
 * the five-seat row at the sheet's foot was 「太丑了」; 「点击后展开一个
 * 菜单，选择最近、工作区啥的」). At the right of the 附栏's one head — the
 * phone's sheet and the Mac's column alike — the seat's own icon, its
 * noun (最近 / 工作区 / 浏览器 / 机器 / 额度: the seat's name, never the
 * page's) and a chevron, with a transparent native `<select>` over all
 * three, so the system draws the picker (iOS's menu, the Mac's pop-up) —
 * `PopupButton`'s own arrangement, never a menu of ours. The icon → noun
 * gap is the face → title gap at the head's left (`gap-in`).
 *
 * `SeatPick` is present inside the 附栏 only; the pages that are drawn
 * both there and on their own screens (`MachineCard`, `QuotaLanding`)
 * read it to drop their own ‹ and face there.
 *
 * ⚠️ 墓碑 (UX-30 §2 → UX-37 §4 → UX-39 §2): the glass capsule at the
 * column's top, then the flat row at the sheet's foot; the page's title
 * as the head (「这个会话碰过的」, the machine's name).
 */
import { createContext, useContext, type ReactNode } from "react";

import { IconBrowser, IconChevronDown, IconDevices, IconFiles, IconQuota, IconRecent } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { SIDE_SEATS, type SideSeat } from "@/hooks/use-session-side";
import { cn } from "@/lib/utils";

export type SeatPickValue = { seat: SideSeat; onSeat: (seat: SideSeat) => void };

/** Present inside the 附栏: which seat is showing, and the way to another. */
export const SeatPick = createContext<SeatPickValue | null>(null);

export function seatName(seat: SideSeat): string {
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

/** Every seat wears its own glyph, the machine's and the account's too
    (the owner: 「每个标题打开了要有头像，也就是对应的 icon」). */
const SEAT_GLYPH: Record<SideSeat, ReactNode> = {
  recent: <IconRecent />,
  workspace: <IconFiles />,
  browser: <IconBrowser />,
  machine: <IconDevices />,
  quota: <IconQuota />,
};

export function SeatTitle({ className }: { className?: string }) {
  const pick = useContext(SeatPick);
  if (!pick) return null;
  return (
    <span data-seat-pick className={cn("relative flex min-w-0 items-center gap-in", className)}>
      <span aria-hidden data-seat-glyph className="grid size-tab-face flex-none place-items-center [&>svg]:size-affordance">
        {SEAT_GLYPH[pick.seat]}
      </span>
      <span aria-hidden data-seat-noun className="min-w-0 truncate text-title-md font-title">
        {seatName(pick.seat)}
      </span>
      <IconChevronDown aria-hidden className="size-affordance flex-none text-muted-foreground" />
      <select
        aria-label={gui.session_side}
        value={pick.seat}
        onChange={(e) => pick.onSeat(e.target.value as SideSeat)}
        className="absolute inset-0 h-full w-full cursor-default appearance-none opacity-0"
      >
        {SIDE_SEATS.map((s) => (
          <option key={s} value={s} data-on={s === pick.seat}>
            {seatName(s)}
          </option>
        ))}
      </select>
    </span>
  );
}
