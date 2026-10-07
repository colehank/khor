/**
 * The 附栏's seats: which one is showing and the way to another
 * (`SeatPick`), and the bar at the right of the 附栏's one head that
 * chooses between them (`SeatBar`, UX-45) — the phone's sheet and the
 * Mac's column alike.
 *
 * `SeatPick` is present inside the 附栏 only; the pages that are drawn
 * both there and on their own screens (`MachineCard`, `QuotaLanding`)
 * read it to drop their own ‹ and face there.
 *
 * ⚠️ 墓碑 (UX-39 §2 → UX-45): **the seat's title as the menu** (the owner
 * 2026-10-06 20:30: 「点击后展开一个菜单，选择最近、工作区啥的」) — the
 * seat's own icon, its noun and a chevron, with a transparent native
 * `<select>` of the five over all three (`SeatTitle`).
 *
 * ⚠️ 墓碑 (UX-30 §2 → UX-37 §4 → UX-39 §2): the glass capsule at the
 * column's top, then the flat row at the sheet's foot; the page's title
 * as the head (「这个会话碰过的」, the machine's name).
 */
import { createContext, useContext, type ReactNode } from "react";

import { GlassCapsule, GlassItem } from "@/components/Glass";
import { IconBrowser, IconDevices, IconFiles, IconMore, IconQuota, IconRecent } from "@/components/icons";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { gui } from "@/gen/catalog";
import { type SideSeat } from "@/hooks/use-session-side";
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

/** The bar's own three; 机器 and 额度 sit in 更多's menu. */
const BAR_SEATS: readonly SideSeat[] = ["recent", "workspace", "browser"];
const MORE_SEATS: readonly SideSeat[] = ["machine", "quota"];

/**
 * **The seat bar** (UX-45, the owner 2026-10-07: 「右边是导航栏…就四个：最
 * 近、工作区、浏览器、更多，更多里面有机器和额度」): at the right of the
 * 附栏's head, a glass capsule of four — 最近 · 工作区 · 浏览器 · 更多 —
 * drawn as the wide face's landing strip draws its seats (the glyph alone,
 * its name the hint, the current one the chosen piece in the brand's deep
 * ink). 更多 opens a small menu of 机器 and 额度, and is lit while one of
 * them is the page. The phone's sheet and the Mac's column alike.
 * ⚠️ 墓碑 (UX-39 §2 → UX-45): the seat's icon · noun · ⌄ over a native
 * picker of the five (`SeatTitle`).
 */
export function SeatBar({ className }: { className?: string }) {
  const pick = useContext(SeatPick);
  if (!pick) return null;
  const moreOn = MORE_SEATS.includes(pick.seat);
  return (
    <GlassCapsule data-seat-bar data-seat-on={pick.seat} role="group" aria-label={gui.session_side} className={cn("flex-none", className)}>
      {BAR_SEATS.map((s) => (
        <GlassItem
          key={s}
          data-seat-item={s}
          on={pick.seat === s}
          aria-current={pick.seat === s ? "page" : undefined}
          aria-label={seatName(s)}
          onClick={() => pick.onSeat(s)}
        >
          {SEAT_GLYPH[s]}
        </GlassItem>
      ))}
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <GlassItem data-seat-item="more" data-seat-more on={moreOn} aria-label={gui.rail_more} tip={false}>
            <IconMore />
          </GlassItem>
        </DropdownMenuTrigger>
        <DropdownMenuContent
          side="bottom"
          align="end"
          sideOffset={8}
          data-seat-more-menu
          onCloseAutoFocus={(e) => e.preventDefault()}
          className="min-w-nav-menu"
        >
          {MORE_SEATS.map((s) => (
            <DropdownMenuItem key={s} data-seat-more-item={s} data-on={pick.seat === s} className="[&_svg]:size-nav-glyph" onSelect={() => pick.onSeat(s)}>
              {SEAT_GLYPH[s]}
              {seatName(s)}
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>
    </GlassCapsule>
  );
}
