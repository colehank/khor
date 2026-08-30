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
 * # Which end is now, said without a word of explanation
 *
 * The strip runs oldest to newest, and nothing on it says so. What says
 * it is free: **the rightmost cell is painted in the same colour as the
 * state word at the end of the row**, a hand's width away — so the live
 * end of the band and the row's current word are the same colour at the
 * same moment, and the eye joins them without being told.
 *
 * That cue was there all along and was invisible for a while, because
 * the 不知道 ink was invisible: a young khor has recorded only a few
 * hours, most of the band was 不知道, and a band that is blank on the
 * left and coloured on the right reads as time running right-to-left —
 * which is what a user concluded. **The blank was doing the arguing, not
 * the axis.** So there is deliberately no legend here (docs/UX.md
 * 装饰性说明归零); if the question comes back now that the line is
 * visible, that judgment was wrong and what it earns is a one-time hint,
 * never a permanent key.
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
  // Nobody was sampling. The flatline.
  //
  // **`--day-unwatched`, and it used to be `--border`.** The app's
  // hairline weight is 1.22 against this ground in both themes, which is
  // not a faint line — it is no line. The three materials this file
  // argues for were three in the data and two on the glass: 不知道 and
  // 那时没有这一行 both read as blank, and with khor young the blank ran
  // most of the way across, so a person read the axis as running
  // right-to-left. The token carries the measurements and the pair of
  // bounds it has to sit between.
  //
  // Read through an inline `var()` like the word branch below, rather
  // than through a Tailwind colour utility: that would need a second
  // entry in `@theme inline` naming the same fact, and the sibling three
  // lines down already resolves its token this way.
  if (mark === "unwatched") {
    return <span className="h-px w-full" style={{ backgroundColor: "var(--day-unwatched)" }} />;
  }
  return (
    <span
      className="h-full w-full"
      style={{ backgroundColor: `var(--state-${mark.word})` }}
    />
  );
}
