import type { QuotaAnswer, QuotaWindow } from "@/api";
import { Ring } from "@/components/Ring";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { gaugeColor } from "@/lib/vitals";
import { ago, until } from "@/words";

/**
 * **The rule above this panel belongs to the panel.** It sits at the
 * foot of the mark's place, under the map, and it draws nothing at all
 * until the first answer is in — so a border owned by the space it
 * occupies would be a line promising a section that has not arrived.
 * An empty box and a full one are the same box; only what is inside
 * knows which.
 */
const SHELL = "border-t";

/**
 * The whole width, and **this used to be `max-w-md`** on the reasoning
 * that a short block of facts should not stretch across a metre of
 * glass. 「下面的 usage 环,水平排呀,现在垂直了多丑,水平空间完全没用」 —
 * the user, 2026-08-28. The cap was what made a horizontal row
 * impossible: inside 448px two windows are two cramped columns, which
 * is the measurement that put them one above the other in the first
 * place. Widened, the arcs are the only thing in this strip and the
 * strip is as wide as the map above it.
 */
const COLUMN = "mx-auto w-full p-4";

/**
 * What the Claude subscription has left — the two windows, as arcs.
 *
 * # This file used to be `UsagePanel`, and what it lost is the point
 *
 * It held two things: these arcs, and a ledger of every day's token
 * counts. 「Khor 主页不显示输出 token 什么的。只显示地图和 AI usage」 —
 * the user, 2026-08-28, with 「AI usage」 confirmed to mean these arcs.
 * So the ledger is gone from the app, **and it was deliberately not
 * re-hung anywhere else** (a lead ruling the same day): 还能用多少 and
 * 花了多少 are two questions, the first is what a person opens a screen
 * to ask, and the second has `khor usage` for as long as anybody wants
 * it. Moving a panel one press deeper is not removing it.
 *
 * The name went with the contents. `UsagePanel` was accurate about the
 * ledger and would have been a lie about what is left — and a file whose
 * name describes what it used to do is the kind of stale reference this
 * repo pays for twice (docs 过期的引用比写错的贵).
 *
 * # Why *these* get arcs when the map's machines get nothing
 *
 * A window is nothing but a denominator and a share of it, so it gets
 * the shape that says 「how full」: a closed arc that carries its own
 * 「满」. That is the same property the star map's bars lacked — a bar
 * says how full something is without saying what, and the user's word
 * for those was 看不懂. Here the denominator is real, it is stated
 * (「5 小时窗口」), and the number is written in the middle.
 *
 * **Neutral rings, no ramp.** A window at 80% is a fact, not a warning,
 * and khor has no threshold to say otherwise: `Strain` exists for memory
 * and disk because the node decides those, and nothing decides this one.
 * Grading it here would be the frontend inventing a judgment — the same
 * thing this app already refused when it left mandala's `severityColor`
 * behind.
 *
 * **The line naming whose login this came through is not a caption, it
 * is the point.** That credential was stored so `claude` could run;
 * khor reading it to answer a second question widens what it was given
 * for, and the person agreed to that widening knowing it. Numbers that
 * appeared with no account behind them would read as khor's own
 * knowledge, which they are not — so the line goes wherever the numbers
 * go.
 */
export function QuotaPanel({ quota }: { quota: QuotaAnswer | null }) {
  // Nothing at all until the first answer is in: neither a reading nor a
  // reason has happened yet, and drawing either would describe a state
  // that does not exist.
  if (!quota) return null;
  const trouble = troubleWord(quota);
  if (trouble) {
    return (
      <section data-quota-panel className={SHELL}>
        <div className={COLUMN}>
          <span data-quota-trouble className="text-aux text-muted-foreground">
            {trouble}
          </span>
        </div>
      </section>
    );
  }
  const q = quota.quota;
  if (!q || q.windows.length === 0) return null;
  return (
    <section data-quota-panel className={SHELL}>
      <div data-quota className={cn(COLUMN, "flex flex-col gap-row")}>
        {/* **Side by side, each window in a column of its own.**
            The stacked version this replaces was measured and its
            measurement was real: at `max-w-md` the two columns came out
            74 and 64 wide with 12 between them, and their reset lines —
            same size, same colour, same baseline, 14 apart — read as one
            sentence,「4 小时后重置 2 天后重置」. **What changed is the
            cap, not the judgment**: the gutter that was 12 is now
            whatever is left of the pane after two cells of content, so
            the words of one window are nowhere near the words of the
            next and no rule has to be drawn to say so.

            `auto-fit` with a floor rather than a fixed count: it is
            horizontal wherever there is room for it and stacks itself on
            a phone — which is the old layout, arrived at by measuring
            instead of by decree, and khor has four window kinds to place
            even though a personal plan shows two. */}
        <div
          className="grid gap-pane"
          style={{ gridTemplateColumns: "repeat(auto-fit, minmax(12rem, 1fr))" }}
        >
          {q.windows.map((w) => (
            <div
              key={w.kind}
              data-quota-window={w.kind}
              className="flex items-center justify-center gap-row"
            >
              {/* The percentage is what this panel exists to say, so it
                  reads at a size of its own — the tier tokens.css
                  describes as being for「a quota arc's centre」and which
                  had never been used at one. */}
              <Ring fraction={w.used_pct / 100} color={gaugeColor(null)} centerClass="text-lg">
                {`${Math.round(w.used_pct)}%`}
              </Ring>
              <span className="flex min-w-0 flex-col">
                {/* 「5 小时窗口」, not 「5 小时」: with the reset line right
                    under it there were two durations touching, one the
                    length of the window and one what is left of it, and
                    nothing on the glass said which was which. */}
                <span className="text-fact">{windowName(w)}</span>
                {/* Only when there is one. A window past its reset has none
                    — the moment it names is behind it, and the reading
                    beside it has already gone back to zero. */}
                {w.resets_at !== null && (
                  <span data-quota-resets className="text-aux text-muted-foreground">
                    {gui.quota_resets(until(w.resets_at * 1000))}
                  </span>
                )}
              </span>
            </div>
          ))}
        </div>
        {/* Whose login this came through, and how old the reading is —
            **one line, because they are one kind of remark**: both
            qualify the arcs above and neither is a number. As two lines
            they were two thirds of the grey text in this corner. The age
            is here for the reason a machine's readings carry one: a
            number with no age beside it is read as the present, and this
            one may be up to five minutes old by design. */}
        <div data-quota-via className="text-aux text-muted-foreground">
          {q.as_of !== null
            ? `${gui.quota_via} · ${cli.vitals_taken(ago(q.as_of * 1000))}`
            : gui.quota_via}
        </div>
      </div>
    </section>
  );
}

/** The name of a window, from its key — the words are the catalog's. */
function windowName(w: QuotaWindow): string {
  switch (w.kind) {
    case "five_hour":
      return gui.quota_five_hour;
    case "seven_day":
      return gui.quota_seven_day;
    case "seven_day_sonnet":
      return gui.quota_seven_day_sonnet;
    case "seven_day_opus":
      return gui.quota_seven_day_opus;
  }
}

/** The sentence for a reason there is no reading. */
function troubleWord(answer: QuotaAnswer): string | null {
  const t = answer.trouble;
  if (!t) return null;
  if (t === "no_login") return gui.quota_no_login;
  if (t === "stale") return gui.quota_stale;
  if (t === "unreadable") return gui.quota_unreadable;
  if (t === "unreachable") return gui.quota_unreachable;
  return gui.quota_cooling(t.cooling.minutes);
}
