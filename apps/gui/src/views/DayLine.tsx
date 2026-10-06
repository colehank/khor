/**
 * **The log hangs under the session pill, always** (UX-36, the owner
 * 2026-10-06; it overrides UX-30 §5, where the line was the 附栏's ruler).
 *
 * `DayLine` is the session's 24 hours (`DayStrip`) flattened into a line
 * two points high: every stretch of the day at its share of the width in
 * its word's ink, 不知道 in its tint, 那时没有这一行 as nothing. **Always
 * present**: while the day is on its way it is a line of `fill` (the
 * app's skeleton language), and a day with no record is still drawn as
 * what it is (a line of 不知道), never left out. It stands in the pill's
 * slot (UX-33 §2), the phone's deck pill and the wide strip's session
 * pills alike; a reload's progress line takes the slot while it runs.
 *
 * `DayPanel` is the day in full — the band at its own height with its
 * readouts and popovers (`DayStrip`, unchanged) and under it the day's
 * stretches as lines, newest first. The wide face opens it from the
 * pill's line through the pill's menu 「今天」 (the pill's own press is
 * the 附栏's); the phone from 最近's first row in the pill's sheet
 * (`DayToday`, UX-37 §5; the pill's tap is the sheet's). The expansion is
 * the session's memory (`use-session-side`'s `day`).
 *
 * ⚠️ 墓碑 (UX-30 §5 → UX-36): the line was the column's ruler, a button
 * under the 附栏's capsule and the phone sheet's, and a tap on it
 * expanded the day there. ⚠️ 墓碑 (2026-09-29 → 10-06): the wide face's
 * day was a band that the front tab's press brought down under the tabs.
 */
import { cli, gui } from "@/gen/catalog";
import { IconChevronDown, IconChevronUp } from "@/components/icons";
import type { DayTotals } from "@/hooks/use-day-totals";
import { cn } from "@/lib/utils";
import { count, stretch } from "@/words";
import { DAY_ROW_ID, DayStrip, dayHasRecord, lay, sentence, type Day, type Stretch } from "@/views/DayStrip";

/** A stretch's ink, the strip's own derivation (`DayStrip`'s `Ink`). */
function inkOf(s: Stretch): string | undefined {
  const m = s.run.mark;
  if (m === "absent") return undefined;
  return m === "unwatched" ? "var(--day-unwatched)" : `var(--state-${m.word})`;
}

function laidOf(runs: Day) {
  return runs === null ? null : lay(runs, runs.reduce((n, r) => n + r.cells, 0) || 1, Date.now());
}

/** The day flattened, as wide as the box it is put in. Nothing to press:
    the pill it hangs under is the press. */
export function DayLine({ runs, band = false, className }: { runs: Day; /** At the band's height (`--day-strip`): 今天's row, UX-37 §5. */ band?: boolean; className?: string }) {
  const laid = laidOf(runs);
  return (
    <span
      data-day-ruler-line
      aria-label={cli.day_title}
      aria-busy={runs === null ? "true" : undefined}
      className={cn("pointer-events-none flex w-full overflow-hidden rounded-full", band ? "h-(--day-strip)" : "h-day-ruler-h", className)}
    >
      {laid === null ? (
        <span data-day-ruler-skeleton className="h-full w-full bg-fill" />
      ) : laid.length === 0 ? (
        // A day the node has nothing for at all is 不知道 end to end — the
        // line is drawn whatever the day says.
        <span data-day-ruler-mark="unwatched" className="h-full w-full" style={{ backgroundColor: "var(--day-unwatched)" }} />
      ) : (
        laid.map((s) => (
          <span
            key={s.i}
            data-day-ruler-mark={typeof s.run.mark === "string" ? s.run.mark : s.run.mark.word}
            className="h-full flex-none"
            style={{ width: s.width, backgroundColor: inkOf(s) }}
          />
        ))
      )}
    </span>
  );
}

/** The day in full. Its ground is the surface it is put on
    (`--side-ground`: the popover's, the sheet's). */
export function DayPanel({ runs, className }: { runs: Day; className?: string }) {
  const laid = laidOf(runs);
  return (
    <div data-detail-day id={DAY_ROW_ID} className={cn("flex flex-col [--day-under:var(--side-ground)]", className)}>
      <DayStrip runs={runs} />
      {runs !== null && !dayHasRecord(runs) ? (
        <p data-day-none className="px-in py-xxs text-callout text-muted-foreground">
          {gui.day_none}
        </p>
      ) : (
        laid && (
          <ol data-day-list className="flex flex-col px-in">
            {laid
              .filter((s) => s.label !== null)
              .reverse()
              .map((s) => (
                <li key={s.i} data-day-line className="flex min-h-touch items-center gap-in border-b text-callout tabular-nums">
                  <span aria-hidden className="size-1.5 flex-none rounded-full" style={{ backgroundColor: inkOf(s) }} />
                  <span className="min-w-0 flex-1 truncate">{sentence(s)}</span>
                </li>
              ))}
          </ol>
        )
      )}
    </div>
  );
}

/** **今天 is 最近's first row on the phone** (UX-37 §5; UX-36 §1 put a
    「今天」 row under the sheet's capsule): the day's heat bar at the
    band's height with its two numbers, 忙碌 · 空闲 (`use-day-totals`,
    counted at the record's minute), and a tap expands the day in full in
    place, under the row; a tap folds it. The pill's own day line is
    unchanged. */
export function DayToday({
  runs,
  totals,
  open,
  onToggle,
}: {
  runs: Day;
  totals: DayTotals | null;
  open: boolean;
  onToggle: () => void;
}) {
  return (
    <div data-day-today-row className="flex-none border-b">
      <button
        type="button"
        data-day-today
        aria-expanded={open}
        aria-controls={open ? DAY_ROW_ID : undefined}
        onClick={onToggle}
        className="flex w-full flex-col gap-in px-(--page-inset) py-in text-left"
      >
        <span className="flex w-full items-baseline gap-in text-callout text-muted-foreground">
          <span className="min-w-0 flex-1 truncate text-foreground">{gui.today}</span>
          {totals && (
            <span data-day-totals className="flex-none tabular-nums">
              {gui.day_busy_idle(totalOf(totals.busy), totalOf(totals.idle))}
            </span>
          )}
          {open ? <IconChevronUp className="size-affordance flex-none self-center text-tertiary" /> : <IconChevronDown className="size-affordance flex-none self-center text-tertiary" />}
        </span>
        <DayLine runs={runs} band />
      </button>
      {open && <DayPanel runs={runs} className="px-[calc(var(--page-inset)-var(--gap-in))] pb-in" />}
    </div>
  );
}

/** A total, never under a minute unless it is none. */
function totalOf(ms: number): string {
  return ms > 0 ? stretch(ms) : count.minutes(0);
}
