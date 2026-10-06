/**
 * **过去24小时, 最近's first row** (UX-39 §4, the owner 2026-10-06
 * 20:30–22:40: 「药丸下面就不绘制 session 的日志了，日志在最近里面，放在
 * 最上面」, 「不可展开」, 「不叫 Today，叫 Last 24h」, 「点击日志条是显示所
 * 点击部分的状态」), on both faces: the day's two totals over its band
 * (`DayStrip`), whose every stretch answers a hover with its readout and
 * a press with its popover (the stretch's hours, its word, its length).
 * Nothing else opens from the row.
 *
 * **A day with no record draws no row** (UX-42, progressive disclosure):
 * 最近 starts at the first opened item. While the day is on its way the
 * row stands with the band's skeleton — what arrives might be a record.
 *
 * ⚠️ 墓碑 (UX-36 → UX-39 §4): `DayLine`, the day flattened into a 2-pt
 * line in every session pill's slot, and `DayPanel`, the day in full
 * hanging from the pill (the menu's 「今天」) or expanding under 今天's row
 * (UX-37 §5). ⚠️ 墓碑 (UX-30 §5 → UX-36): the line as the 附栏's ruler.
 * ⚠️ 墓碑 (2026-09-29 → 10-06): the wide face's day as a band under the
 * tabs, brought down by the front tab's press.
 */
import { gui } from "@/gen/catalog";
import type { DayTotals } from "@/hooks/use-day-totals";
import { count, stretch } from "@/words";
import { DayStrip, dayHasRecord, type Day } from "@/views/DayStrip";

export function DayToday({ runs, totals }: { runs: Day; totals: DayTotals | null }) {
  if (runs !== null && !dayHasRecord(runs)) return null;
  const said = totals ? gui.day_busy_idle(totalOf(totals.busy), totalOf(totals.idle)) : null;
  return (
    // `data-detail-day`: the session's day, wherever it is drawn (the
    // gates find the band by it).
    <div data-day-today-row data-day-today data-detail-day className="flex flex-none flex-col gap-in border-b px-(--page-inset) py-in">
      <span className="flex w-full items-baseline gap-in text-callout text-muted-foreground">
        <span className="min-w-0 flex-1 truncate text-foreground">{gui.day_last_24h}</span>
        {said && (
          <span data-day-totals className="flex-none tabular-nums">
            {said}
          </span>
        )}
      </span>
      <DayStrip runs={runs} />
    </div>
  );
}

/** A total, never under a minute unless it is none. */
function totalOf(ms: number): string {
  return ms > 0 ? stretch(ms) : count.minutes(0);
}
