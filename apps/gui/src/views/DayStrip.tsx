import type { DayRun } from "@/api";

/**
 * One session's last 24 hours, as a band across the bottom of its row.
 *
 * **Called a day strip and not a strip**, because `SessionsList` already
 * uses that word for a row — "the row is a strip, not a button itself".
 * A second meaning for it in the same pane is how the next person greps
 * for one and reads the other.
 *
 * # Why it is full-bleed
 *
 * A strip is a time axis, and axes only compare if they line up. Inset
 * to the text column it would still be readable on its own, and reading
 * two rows against each other — *was this one blocked while that one was
 * working?* — would silently be wrong for any row whose indent differs.
 * Edge to edge, every row on the list shares one axis by construction.
 *
 * # Three materials, because two of the three are not colours
 *
 * The six words are painted in their own tokens and nothing else is
 * (docs/UX.md 状态呈现: 六词之外无第七词). The two nothings
 * (`khor_core::DayCell`) had to be told apart from the words *and from
 * each other* without inventing a seventh hue, so what separates them is
 * **shape**:
 *
 * - a word — the full height of the band, solid, in its own colour.
 * - 不知道 (`unwatched`, nobody was sampling) — a hairline down the
 *   middle of the band in the app's quietest ink. A flatline: signal
 *   absent, which is exactly what it means.
 * - 那时没有这一行 (`absent`, khor was sampling and this session was not
 *   there) — nothing at all, the row's own ground.
 *
 * **Not one tint at three opacities**, which is what a lazy version of
 * this would be and is the way the two nothings would quietly fold back
 * into one on screen after everything below the glass worked to keep
 * them apart. Full block, hairline, nothing: three different amounts of
 * substance, legible before colour is read at all.
 *
 * # It paints, it does not decide
 *
 * The runs arrive merged to the cell count this list asked for, and
 * which cell won a merge was `khor_core::DayCell::rank`'s answer. There
 * is no ordering, no bucketing and no fallback word in this file — a
 * second copy of that judgment here is how a strip and the row above it
 * would come to disagree about the same session.
 */
export function DayStrip({ runs }: { runs: DayRun[] | undefined }) {
  if (!runs || runs.length === 0) return null;
  const total = runs.reduce((n, r) => n + r.cells, 0);
  if (total === 0) return null;
  return (
    <span
      data-day-strip
      aria-hidden="true"
      className="pointer-events-none absolute inset-x-0 bottom-0 flex h-1 overflow-hidden"
    >
      {runs.map((run, i) => (
        <span
          // Runs have no identity of their own — they are a slice of a
          // timeline, and the slice at position i is the same thing
          // frame to frame even when its width changes.
          key={i}
          data-day-mark={typeof run.mark === "string" ? run.mark : run.mark.word}
          // A grid rather than a plain box so the hairline case has
          // somewhere to be centred; a word simply fills it.
          className="grid h-full place-items-center"
          style={{ width: `${(run.cells / total) * 100}%` }}
        >
          <Mark mark={run.mark} />
        </span>
      ))}
    </span>
  );
}

/**
 * What one run is made of.
 *
 * The union is discriminated in the type (`DayMark` is
 * `{ word } | "unwatched" | "absent"`), so a word can only be reached
 * through the branch that has one — which is the point of it having that
 * shape. Spelled as one flat string, `var(--state-unwatched)` would
 * resolve to nothing and paint transparent, folding 不知道 into
 * 那时没有这一行 in CSS after the node worked to keep them apart.
 */
function Mark({ mark }: { mark: DayRun["mark"] }) {
  // Sampled, and this session was not on the list. The band's own
  // ground, and deliberately no element: "nothing here" is the one of
  // the three that should cost no ink at all.
  if (mark === "absent") return null;
  // Nobody was sampling. The flatline — `bg-border` is the app's
  // hairline weight, the quietest ink it has, and not one of the six.
  if (mark === "unwatched") return <span className="h-px w-full bg-border" />;
  return (
    <span
      className="h-full w-full"
      style={{ backgroundColor: `var(--state-${mark.word})` }}
    />
  );
}
