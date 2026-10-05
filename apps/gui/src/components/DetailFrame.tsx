/**
 * **A detail page's content: the pane's width, capped and centred** (UX-22,
 * the owner 2026-10-05: with the list put away, 机器 and 额度 kept their
 * old width and the right side stood empty, while 工作区 and 浏览器 laid
 * themselves out to the window). One frame for every detail landing: it
 * fills the pane it is in and re-lays out with it — on a resize, on the
 * list going away and coming back — up to `--detail-max` (the 960 ruling,
 * F6.6/W24: past it a label stands too far from its bar), and past that it
 * stays in the middle with the room shared on both sides. A page whose
 * content is the page — a file, a site — has no cap and fills it
 * (工作区, 浏览器), which is the same rule with no maximum.
 *
 * Its own `--page-inset` on both sides, so the content's edge is the
 * page's line at any width. R0's title follows it (`Region` `capped`).
 *
 * ⚠️ 墓碑 (F6.6 → UX-22): 「capped and stays on the page's left line — the
 * room to its right is the window's」.
 */
import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

export function DetailFrame({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      data-detail-frame
      className={cn("mx-auto w-full max-w-[calc(var(--detail-max)+2*var(--page-inset))] px-page", className)}
      {...props}
    />
  );
}
