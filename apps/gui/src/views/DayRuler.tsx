/**
 * **The log does not take a seat: it is the column's ruler** (UX-30 §5).
 *
 * A heat line two points high directly under the 附栏's capsule — the
 * session's 24 hours (`DayStrip`) flattened: every stretch of the day at
 * its share of the width in its word's ink, 不知道 in its tint, 那时没有
 * 这一行 as nothing. **Always present**: while the day is on its way it is
 * a line of `fill` (the app's skeleton language), and a day with no
 * record is still drawn as what it is (a line of 不知道), never left out
 * — the column's head keeps one geometry whatever the day says.
 *
 * A tap expands it in place into the full day view **over the seat's
 * content**: the band at its own height with its readouts and popovers
 * (`DayStrip`, unchanged), and under it the day's stretches as lines,
 * newest first. A tap folds it. The expansion is remembered with the
 * column (`use-session-side`), the way the old band's switch was
 * (`use-day-shown`, which the narrow head still uses).
 *
 * ⚠️ 墓碑 (2026-09-29 → 10-06): the wide face's day was a band that the
 * front tab's press brought down under the tabs, floating over the
 * session. That press opens the column now, and the day lives here.
 */
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { DAY_ROW_ID, DayStrip, dayHasRecord, lay, sentence, type Day, type Stretch } from "@/views/DayStrip";

/** A stretch's ink, the strip's own derivation (`DayStrip`'s `Ink`). */
function inkOf(s: Stretch): string | undefined {
  const m = s.run.mark;
  if (m === "absent") return undefined;
  return m === "unwatched" ? "var(--day-unwatched)" : `var(--state-${m.word})`;
}

export function DayRuler({ runs, open, onToggle }: { runs: Day; open: boolean; onToggle: () => void }) {
  const laid = runs === null ? null : lay(runs, runs.reduce((n, r) => n + r.cells, 0) || 1, Date.now());
  return (
    <div data-day-ruler-row className="relative flex-none">
      {/* The line, and the finger's room over and under it: the hit box
          is the row's padding, the ink is two points. */}
      <button
        type="button"
        data-day-ruler
        aria-label={cli.day_title}
        aria-expanded={open}
        aria-controls={open ? DAY_ROW_ID : undefined}
        aria-busy={runs === null ? "true" : undefined}
        onClick={onToggle}
        className="block w-full px-nav-gap py-xxs outline-none focus-visible:[&>span]:outline-2 focus-visible:[&>span]:outline-offset-2 focus-visible:[&>span]:outline-ring"
      >
        <span data-day-ruler-line className="flex h-day-ruler-h w-full overflow-hidden rounded-full">
          {laid === null ? (
            <span data-day-ruler-skeleton className="h-full w-full bg-fill" />
          ) : laid.length === 0 ? (
            // A day the node has nothing for at all is 不知道 end to end —
            // the ruler is drawn whatever the day says.
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
      </button>
      {open && (
        // The day in full, over the seat's content: an absolute panel
        // hanging from the ruler's row, so the page under it does not
        // move by a pixel (the band's own rule, 2026-09-29).
        <div
          data-detail-day
          id={DAY_ROW_ID}
          // The band's axis is the ruler's: the strip keeps its own `gap-in`
          // inset, so the panel adds the rest of the capsule's `nav-gap`.
          className={cn(
            "absolute inset-x-0 top-full z-30 flex max-h-[60dvh] flex-col overflow-y-auto border-b bg-card px-[calc(var(--nav-gap)-var(--gap-in))] pb-in",
            "[--day-under:var(--card)]",
          )}
        >
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
      )}
    </div>
  );
}
