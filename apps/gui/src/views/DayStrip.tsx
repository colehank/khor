import { useRef, useState } from "react";

import type { DayRun } from "@/api";
import { gui } from "@/gen/catalog";
import { word } from "@/words";

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
 * # One axis, three tiers — and the third is the absence of ink
 *
 * The six words are painted in their own tokens and nothing else is
 * (docs/UX.md 状态呈现: 六词之外无第七词). The two nothings
 * (`khor_core::DayCell`) have to be told apart from the words *and from
 * each other* without inventing a seventh hue:
 *
 * - a word — a block the full height of the band, in its own colour.
 * - 不知道 (`unwatched`, nobody was sampling) — a block of the same
 *   height, in a quiet tint of the page's own ink. No hue, because it
 *   is not a state.
 * - 那时没有这一行 (`absent`, khor was sampling and this session was not
 *   there) — **no element at all**, the row's own ground.
 *
 * # Why this is not the thing this comment used to forbid
 *
 * It used to say "not one tint at three opacities" and drew 不知道 as a
 * **hairline** to make sure. That defended the right thing — the two
 * nothings must not differ only by how light they are — and it is still
 * defended here, because **one of them has no ink at all**. What the
 * hairline bought on top of that was not distinction, it was a second
 * kind of object: a user reading a real strip said 「左边有一条细线,
 * 右边才是 bar」 — a line, and a bar. Two nouns for one time axis, and
 * the axis is the thing the strip exists to be.
 *
 * So the geometry went back to one, and the separation moved into
 * weight, where there is room for it: 不知道 is ×2.9 lighter than the
 * faintest word (`--day-unwatched` carries the measurements). The pair
 * that has to stay apart is now 不知道 and **空闲** — the one word left
 * without a hue — and the gate measures exactly that gap rather than
 * trusting this paragraph.
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
/** The strip's whole span, in minutes: the node's `Day` is a rolling
    day (`khor_node::day`), and every run is a share of it. */
const DAY_MINUTES = 24 * 60;

/** A clock reading, `HH:MM`, for the hover. */
function clock(ms: number): string {
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/**
 * What the pointer is over: the **moment** under the cursor, and the
 * word there (2026-09-02: 「鼠标悬浮在状态bar上面,应该显示当时的时间
 * 和状态」). The first spelling (2026-09-01) printed the run's whole
 * span through the browser's own `title` — and its 1.5s delay read as
 * no hover at all, which is how the same request arrived twice. This
 * one is immediate and follows the cursor: the axis is linear, so the
 * x under the pointer *is* a time of day.
 *
 * **The clock is read at hover, not at fetch.** The strip's right edge
 * is "now" as of the last poll, and the poll is a few seconds — a
 * reading that lands inside one poll is inside one minute, and the
 * readout prints minutes. Carrying the fetch time out of the hook would
 * be a second clock for a difference the format cannot show.
 *
 * An absent run has nothing to say and takes no pointer; the word for a
 * stretch khor was not watching is its own (`day_unwatched`), never a
 * seventh state. The ` · ` join is punctuation, not copy — the words on
 * either side of it are the catalog's.
 */
type Peek = { x: number; text: string };

function labelOf(mark: DayRun["mark"]): string | null {
  if (mark === "absent") return null;
  return mark === "unwatched" ? gui.day_unwatched : word(mark.word);
}

export function DayStrip({ runs }: { runs: DayRun[] | undefined }) {
  const box = useRef<HTMLSpanElement>(null);
  const [peek, setPeek] = useState<Peek | null>(null);
  if (!runs || runs.length === 0) return null;
  const total = runs.reduce((n, r) => n + r.cells, 0);
  if (total === 0) return null;
  const read = (e: React.MouseEvent, label: string) => {
    const rect = box.current?.getBoundingClientRect();
    if (!rect || rect.width === 0) return;
    const frac = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
    const at = Date.now() - (1 - frac) * DAY_MINUTES * 60_000;
    // Clamped so the readout never hangs past the pane at either end.
    const x = Math.min(Math.max(e.clientX - rect.left, 28), rect.width - 28);
    setPeek({ x, text: `${clock(at)} · ${label}` });
  };
  return (
    <span
      ref={box}
      data-day-strip
      // **Only the runs take the pointer**, and each says what it is.
      // The strip is four pixels tall, so every run also reaches up
      // behind the row with an invisible hit box (`before:`): a person
      // aiming at the bar should not need a steady hand to hit it.
      className="pointer-events-none absolute inset-x-0 bottom-0 flex h-1 overflow-visible"
    >
      {runs.map((run, i) => {
        const label = labelOf(run.mark);
        return (
        <span
          // Runs have no identity of their own — they are a slice of a
          // timeline, and the slice at position i is the same thing
          // frame to frame even when its width changes.
          key={i}
          data-day-mark={typeof run.mark === "string" ? run.mark : run.mark.word}
          // Every tier that has ink fills this box; the one that has
          // none renders nothing into it.
          className={
            label === null
              ? "grid h-full place-items-center"
              : "pointer-events-auto relative grid h-full place-items-center before:absolute before:inset-x-0 before:-top-3 before:bottom-0 before:content-['']"
          }
          style={{ width: `${(run.cells / total) * 100}%` }}
          onMouseMove={label === null ? undefined : (e) => read(e, label)}
          onMouseLeave={label === null ? undefined : () => setPeek(null)}
        >
          <Mark mark={run.mark} />
        </span>
        );
      })}
      {/* The readout, above the band where the cursor is not covering
          it. Its own element rather than the OS title: immediate, and
          it moves with the pointer. `pointer-events` stays off it so it
          never steals the hover that summoned it. */}
      {peek && (
        <span
          data-day-peek
          className="absolute bottom-full z-10 mb-1 -translate-x-1/2 rounded-md border bg-popover px-1.5 py-0.5 text-caption whitespace-nowrap text-popover-foreground shadow-sm"
          style={{ left: peek.x }}
        >
          {peek.text}
        </span>
      )}
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
  // Nobody was sampling — the same block a word gets, in ink with no
  // hue. The token carries what it has to sit between and why (a floor
  // for a filled block, and a factor below 空闲); this file's head has
  // why it stopped being a hairline.
  //
  // Read through an inline `var()` like the word branch below, rather
  // than through a Tailwind colour utility: that would need a second
  // entry in `@theme inline` naming the same fact, and the sibling three
  // lines down already resolves its token this way.
  if (mark === "unwatched") {
    return <span className="h-full w-full" style={{ backgroundColor: "var(--day-unwatched)" }} />;
  }
  return (
    <span
      className="h-full w-full"
      style={{ backgroundColor: `var(--state-${mark.word})` }}
    />
  );
}
