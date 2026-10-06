import { gui } from "@/gen/catalog";

/** **A bell rang here while it was off screen** (the terminal audit, item
    67, `lib/bell-marks`): the row's unread badge without a number, the
    one mark that already says "this wants you" — the same `badge` fill,
    no new colour. A tab wears it before its name, a list row where its
    unread count would stand. */
export function BellMark() {
  return <span data-bell-rang role="img" aria-label={gui.bell_rang} className="size-2 flex-none rounded-full bg-badge" />;
}
