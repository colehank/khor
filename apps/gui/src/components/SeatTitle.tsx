/**
 * **The seat's title is the menu** (UX-39, the owner 2026-10-06 20:30:
 * the five-seat row at the sheet's foot was 「太丑了」; 「点击后展开一个
 * 菜单，选择最近、工作区啥的」). Inside the 附栏 — the phone's sheet and
 * the Mac's column alike — a seat's head says its noun (本会话, the
 * directory, the host, the machine, the account) and that noun is the
 * way to the other seats: a chevron after it, and a transparent native
 * `<select>` over both, so the system draws the picker (iOS's menu, the
 * Mac's pop-up) — `PopupButton`'s own arrangement, never a menu of ours.
 * Outside the 附栏 (no `SeatPick` around) the title is plain text, so the
 * same heads keep working on the narrow face's own screens.
 *
 * ⚠️ 墓碑 (UX-30 §2 → UX-37 §4 → here): the glass capsule at the column's
 * top, then the flat row at the sheet's foot.
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

/** The seat's own glyph, before its title (the owner: 「每个标题打开了要
    有头像，也就是对应的 icon」). A head that already wears a face (the
    machine's, the account's) hands `icon={false}` and keeps its own. */
const SEAT_GLYPH: Record<SideSeat, ReactNode> = {
  recent: <IconRecent />,
  workspace: <IconFiles />,
  browser: <IconBrowser />,
  machine: <IconDevices />,
  quota: <IconQuota />,
};

export function SeatTitle({ children, className, icon = true }: { children?: ReactNode; className?: string; icon?: boolean }) {
  const pick = useContext(SeatPick);
  if (!pick) return <span className={cn("min-w-0 truncate text-title-md font-title", className)}>{children}</span>;
  return (
    <span data-seat-pick className={cn("relative flex min-w-0 items-center gap-in", className)}>
      {icon && (
        <span aria-hidden data-seat-glyph className="grid size-tab-face flex-none place-items-center [&>svg]:size-affordance">
          {SEAT_GLYPH[pick.seat]}
        </span>
      )}
      {/* **The seat's name, not the page's** (the owner 2026-10-06 21:13:
          「机器标题还叫 MacLab？不就是 machine 吗，一致啊」): the five
          heads say the five nouns; what the page is about is said inside. */}
      <span aria-hidden className="min-w-0 truncate text-title-md font-title">
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
