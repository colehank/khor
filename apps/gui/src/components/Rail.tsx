/**
 * **The wide face's rail** (the redesign, 2026-10-02, the owner: the
 * wide window keeps today's icon rail). Icons only, on the one ground
 * with a hairline to its right; the landings in the user's order
 * (`lib/nav-order` — all five, nothing folds here), then 更多 and this
 * machine at the foot. The open landing's glyph takes the brand's deep
 * ink with its solid piece filled (`rail-on-ink`, `app.css`), never a
 * block behind it.
 *
 * **Pressing the open landing again folds its list away**, and pressing
 * it (or any landing) once more brings the list back — the shell does
 * the folding; this only says which was pressed.
 *
 * The narrow face has no rail: its navigation floats (`FloatNav`).
 */
import type { ReactNode } from "react";

import { MachineAvatar } from "@/components/Avatar";
import { IconMore } from "@/components/icons";
import { WINDOW_DRAG } from "@/components/shell";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { Avatar } from "@/gen/bindings/Avatar";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

export type RailSeat = { key: string; name: string; glyph: ReactNode; badge?: number };

const BTN =
  "relative grid h-rail-btn w-rail-btn flex-none place-items-center rounded-lg text-rail-ink hover:bg-fill " +
  "focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring [&>svg]:size-rail-glyph";
const ON = "text-[var(--rail-on-ink)]";

function Item({
  label,
  on,
  mark,
  onClick,
  children,
}: {
  label: string;
  on: boolean;
  mark: Record<string, string | boolean | undefined>;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          data-slot="button"
          data-rail-item
          data-on={on}
          aria-label={label}
          aria-current={on ? "page" : undefined}
          onClick={onClick}
          className={cn(BTN, on && ON)}
          {...mark}
        >
          {children}
        </button>
      </TooltipTrigger>
      <TooltipContent side="right" data-rail-tip>
        {label}
      </TooltipContent>
    </Tooltip>
  );
}

export function Rail({
  seats,
  active,
  moreOn,
  onGo,
  onMore,
  me,
  meOn,
  onMe,
}: {
  seats: RailSeat[];
  /** The open landing, or null while a 「…」 page is up. */
  active: string | null;
  /** Whether 更多 holds the page on screen. */
  moreOn: boolean;
  onGo: (key: string) => void;
  onMore: () => void;
  me: { face: Avatar | null; paused: boolean } | null;
  meOn: boolean;
  onMe: () => void;
}) {
  return (
    <nav
      data-rail
      aria-label={gui.nav}
      {...WINDOW_DRAG}
      className="flex w-rail flex-none flex-col items-center pt-xxs pb-3.5 select-none"
    >
      <div className="flex flex-col items-center gap-1.5">
        {seats.map((s) => (
          <Item key={s.key} label={s.name} on={active === s.key} mark={{ "data-landing": s.key }} onClick={() => onGo(s.key)}>
            {s.glyph}
            {s.badge !== undefined && s.badge > 0 && (
              <span className="absolute top-0 right-0 min-w-4 rounded-full bg-badge px-1 text-center text-caption leading-4 tabular-nums text-badge-foreground">
                {s.badge}
              </span>
            )}
          </Item>
        ))}
      </div>
      <span className="flex-1" />
      <Item label={gui.rail_more} on={moreOn} mark={{ "data-rail-more": "" }} onClick={onMore}>
        <IconMore />
      </Item>
      <Item
        label={gui.identity}
        on={meOn}
        mark={{ "data-rail-mark": "", "data-paused": me?.paused ? "" : undefined }}
        onClick={onMe}
      >
        <MachineAvatar face={me?.face ?? null} className="size-ctl-md" />
      </Item>
    </nav>
  );
}
